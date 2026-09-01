use std::collections::HashMap;
use std::time::Duration;

use crate::TorErrors;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, Proxy, RequestBuilder};
use serde::{Deserialize, Serialize};

/// Supported HTTP methods
#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
}

/// HTTP response structure compatible with FFI
#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, Vec<String>>,
    pub body: String,
}

/// HTTP request parameters
#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestParams {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout_ms: Option<u64>,
    /// When `Some(true)`, accept self-signed or otherwise invalid TLS
    /// certificates. Defaults to `false`. Intended for use cases like
    /// Tor v3 hidden services, where the `.onion` address already
    /// authenticates the endpoint and the upstream host typically
    /// presents a self-signed cert (e.g. LND REST).
    pub trust_invalid_certs: Option<bool>,
}

fn build_socks_proxy_url(socks_proxy: &str) -> String {
    format!("socks5h://{}", socks_proxy)
}

fn collect_response_headers(headers: &HeaderMap) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers {
        result
            .entry(name.as_str().to_ascii_lowercase())
            .or_default()
            .push(value.to_str().unwrap_or_default().to_string());
    }
    result
}

/// Makes an HTTP request through the Tor SOCKS proxy using reqwest
pub async fn make_http_request_async(
    params: HttpRequestParams,
    socks_proxy: String,
) -> Result<HttpResponse, TorErrors> {
    // Create client with proxy
    let mut builder = Client::builder()
        .proxy(
            Proxy::all(build_socks_proxy_url(&socks_proxy))
                .map_err(|e| TorErrors::TcpStreamError(format!("Failed to create proxy: {}", e)))?,
        )
        .timeout(Duration::from_millis(params.timeout_ms.unwrap_or(30000)));

    if params.trust_invalid_certs.unwrap_or(false) {
        builder = builder.danger_accept_invalid_certs(true);
    }

    let client = builder
        .build()
        .map_err(|e| TorErrors::TcpStreamError(format!("Failed to create client: {}", e)))?;

    // Create request builder based on method
    let method = match params.method {
        HttpMethod::GET => Method::GET,
        HttpMethod::POST => Method::POST,
        HttpMethod::PUT => Method::PUT,
        HttpMethod::DELETE => Method::DELETE,
        HttpMethod::HEAD => Method::HEAD,
        HttpMethod::OPTIONS => Method::OPTIONS,
    };

    let mut req_builder: RequestBuilder = client.request(method, &params.url);

    // Add headers if provided
    if let Some(headers) = params.headers {
        for (name, value) in headers {
            req_builder = req_builder.header(name, value);
        }
    }

    // Add body if provided
    if let Some(body) = params.body {
        req_builder = req_builder.body(body);
    }

    // Send request
    let response = req_builder.send().await?;
    let status_code = response.status().as_u16();
    let headers = collect_response_headers(response.headers());
    let body = response.text().await?;
    Ok(HttpResponse {
        status_code,
        headers,
        body,
    })
}

/// Synchronous wrapper for make_http_request_async
pub fn make_http_request(
    params: HttpRequestParams,
    socks_proxy: String,
) -> Result<HttpResponse, TorErrors> {
    use crate::ensure_runtime;

    ensure_runtime().block_on(async { make_http_request_async(params, socks_proxy).await })
}

#[cfg(test)]
mod tests {
    use super::{build_socks_proxy_url, collect_response_headers};
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
    use std::collections::HashMap;

    #[test]
    fn builds_remote_dns_socks_proxy_url() {
        assert_eq!(
            build_socks_proxy_url("127.0.0.1:9050"),
            "socks5h://127.0.0.1:9050"
        );
    }

    #[test]
    fn collects_lowercase_multi_value_response_headers() {
        let mut headers = HeaderMap::new();
        headers.append(SET_COOKIE, HeaderValue::from_static("a=1"));
        headers.append(SET_COOKIE, HeaderValue::from_static("b=2"));

        assert_eq!(
            collect_response_headers(&headers).get("set-cookie"),
            Some(&vec!["a=1".to_string(), "b=2".to_string()])
        );
    }

    #[test]
    fn serializes_the_public_response_shape() {
        let response = super::HttpResponse {
            status_code: 404,
            headers: HashMap::new(),
            body: "missing".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"statusCode":404,"headers":{},"body":"missing"}"#
        );
    }
}
