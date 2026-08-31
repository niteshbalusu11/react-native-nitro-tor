use std::collections::HashMap;
use std::convert::TryInto;

use tor::http_client::{HttpMethod, HttpRequestParams, make_http_request};
use tor::{TorService, TorServiceParam};

fn main() {
    // Configure logging

    println!("Starting Tor service...");

    // Start Tor service with a temporary data directory
    let service: TorService = TorServiceParam {
        socks_port: Some(19054),
        data_dir: String::from("/tmp/tor_test"),
        bootstrap_timeout_ms: Some(60000), // 60 seconds for bootstrap
    }
    .try_into()
    .expect("Failed to initialize Tor service");

    println!("Converting to owned service...");

    // Convert to owned service (this will wait for bootstrap to complete)
    let mut owned_node = service
        .into_owned_node()
        .expect("Failed to bootstrap Tor service");

    println!("Tor service ready on port {}", owned_node.socks_port);

    // Test HTTP GET request (with HTTPS now supported)
    println!("Testing HTTPS GET request...");
    let get_params = HttpRequestParams {
        url: "https://httpbin.org/get".to_string(),
        method: HttpMethod::GET,
        headers: None,
        body: None,
        timeout_ms: Some(30000), // 30 seconds timeout
    };

    let socks_proxy = format!("127.0.0.1:{}", owned_node.socks_port);

    match make_http_request(get_params, socks_proxy.clone()) {
        Ok(response) => {
            println!("GET Request Status: {}", response.status_code);
            println!("GET Response Body: {:?}", response.body);

            if let Some(error) = response.error {
                println!("Error: {}", error);
            }
        }
        Err(e) => {
            println!("GET Request failed: {:?}", e);
        }
    }

    println!("\nTesting onion GET request...");
    let onion_get_params = HttpRequestParams {
        url: "http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion".to_string(),
        method: HttpMethod::GET,
        headers: None,
        body: None,
        timeout_ms: Some(30000),
    };

    match make_http_request(onion_get_params, socks_proxy.clone()) {
        Ok(response) => {
            println!("Onion GET Status: {}", response.status_code);
            println!("Onion GET Error: {:?}", response.error);
            println!(
                "Onion GET Body Prefix: {:?}",
                &response.body.chars().take(120).collect::<String>()
            );
        }
        Err(e) => {
            println!("Onion GET failed: {:?}", e);
        }
    }

    // Test HTTPS POST request with headers and body
    println!("\nTesting HTTPS POST request...");
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("User-Agent".to_string(), "Tor-Test-Client/1.0".to_string());

    let post_params = HttpRequestParams {
        url: "https://httpbin.org/post".to_string(),
        method: HttpMethod::POST,
        headers: Some(headers),
        body: Some(r#"{"test": "data", "from": "tor"}"#.to_string()),
        timeout_ms: Some(30000), // 30 seconds timeout
    };

    match make_http_request(post_params, socks_proxy) {
        Ok(response) => {
            println!("POST Request Status: {}", response.status_code);
            println!(
                "Response contains request data: {}",
                response.body.contains("\"test\": \"data\"")
            );

            if let Some(error) = response.error {
                println!("Error: {}", error);
            } else {
                // Print a portion of the response body

                println!("Response Body: {:?}", response.body);
            }
        }
        Err(e) => {
            println!("POST Request failed: {:?}", e);
        }
    }

    // Shutdown the Tor service
    println!("\nShutting down Tor service...");
    match owned_node.shutdown() {
        Ok(_) => println!("Tor service successfully shutdown"),
        Err(e) => println!("Error shutting down Tor service: {:?}", e),
    }
}
