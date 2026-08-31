use std::collections::HashMap;
use std::time::Duration;

use crate::TorErrors;
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
pub struct HttpResponse {
    pub status_code: u16,
    pub body: String,
    pub error: Option<String>,
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
    match req_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            match response.text().await {
                Ok(body) => Ok(HttpResponse {
                    status_code: status,
                    body,
                    error: None,
                }),
                Err(e) => Ok(HttpResponse {
                    status_code: status,
                    body: String::new(),
                    error: Some(format!("Failed to read response body: {}", e)),
                }),
            }
        }
        Err(e) => Ok(HttpResponse {
            status_code: 0,
            body: String::new(),
            error: Some(format!("Request failed: {}", e)),
        }),
    }
}

/// Synchronous wrapper for make_http_request_async
pub fn make_http_request(
    params: HttpRequestParams,
    socks_proxy: String,
) -> Result<HttpResponse, TorErrors> {
    use crate::ensure_runtime;

    ensure_runtime()
        .lock()
        .unwrap()
        .block_on(async { make_http_request_async(params, socks_proxy).await })
}

#[cfg(test)]
mod tests {
    use super::build_socks_proxy_url;

    #[test]
    fn builds_remote_dns_socks_proxy_url() {
        assert_eq!(
            build_socks_proxy_url("127.0.0.1:9050"),
            "socks5h://127.0.0.1:9050"
        );
    }
}
