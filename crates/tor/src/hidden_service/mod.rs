use crate::tcp_stream::DataObserver;
use crate::{ensure_runtime, TorErrors};
use base64::engine::general_purpose;
use base64::Engine;
use logger::log::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use httparse::Request;
use serde_json::json;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::sync::RwLock;

pub type HiddenServiceDataHandler = Box<dyn DataObserver + Send + Sync + 'static>;

pub struct HiddenServiceHandler {
    port: u16,
    data_handler: Arc<RwLock<Option<HiddenServiceDataHandler>>>,
}

impl HiddenServiceHandler {
    pub fn new(port: u16) -> Result<Self, TorErrors> {
        Ok(HiddenServiceHandler {
            port,
            data_handler: Arc::new(RwLock::new(None)),
        })
    }
    pub fn set_data_handler<F>(&self, callback: F) -> Result<(), TorErrors>
    where
        F: DataObserver + Send + Sync + 'static,
    {
        ensure_runtime().lock().unwrap().block_on(async move {
            let data_clone = self.data_handler.clone();
            let mut data_write = data_clone.write().await;
            *data_write = Some(Box::new(callback));
        });
        Ok(())
    }

    pub fn start_http_listener(&mut self) -> Result<(), TorErrors> {
        let cb_clone = self.data_handler.clone();
        let port = self.port;
        ensure_runtime().lock().unwrap().spawn(async move {
            let listener = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                port,
            )))
                .await
                .unwrap();
            info!(
                "Started HTTP listener & waiting for connection on port {}",
                port
            );
         loop{
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    info!("New client connection established from addr {:?}", addr);
                    let (mut rx, mut tx) = stream.split();

                    let mut buffer = vec![0; 4096];
                    let mut position = 0;

                    trace!("--> awaiting reading to end.");
                    loop {
                        // drop header after checking http request is complete
                        // TODO: httpparse api means it has to be dropped to avoid RwLock gymnastics on the buffer
                        // Maybe it's more efficient to actually wrap buffer with an RwLock vs this drop and reparse ?
                        {
                            let mut headers = [httparse::EMPTY_HEADER; 16];
                            let mut req = Request::new(&mut headers);
                            match req.parse(&buffer) {
                                Ok(status) => {
                                    status.is_complete();
                                    debug!("<-- parsed request!");
                                    break;
                                }
                                Err(e) => {
                                    match e {
                                        // http parse expects new line to be read before sending it the buffer
                                        // so ignore this error here
                                        httparse::Error::Token => { trace!("got http parse token error, ignoring.") }
                                        _ => {
                                            error!("http Parsing error {:#?}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // resize buffer as needed
                        if position == buffer.len() {
                            buffer.resize(position * 2, 0)
                        }
                        // read
                        let read_size = rx.read(&mut buffer[position..]).await.unwrap();
                        trace!("Read buffer size {} call number {}", read_size, position);
                        // break on some data but 0 data read (peer terminated connection)
                        if position >= 1 && read_size == 0 {
                            warn!("<- peer terminated connection detected");
                            break;
                        }
                        position += read_size;
                    }
                    trace!("-- awaiting reading to end.");

                    trace!("-> parse body");
                    let mut headers = [httparse::EMPTY_HEADER; 16];
                    let mut req = Request::new(&mut headers);
                    let status = req.parse(&buffer).unwrap();
                    let body = {
                        if status.is_complete() {
                            let start_index = status.unwrap();
                            let end_index = position;
                            trace!("<- parse body from {} to {}", start_index, end_index);
                            let mut buf = String::new();
                            general_purpose::STANDARD.encode_string(&buffer[start_index..end_index], &mut buf);
                            buf
                        } else {
                            trace!("<- parse body non complete request");
                            String::from("")
                        }
                    };
                    trace!("-- parse body");
                    trace!("-> parse header");
                    // FIXME a result of succesfull parsed headers here
                    let headers_map: Box<HashMap<String, String>> = Box::new(
                        req.headers
                            .into_iter()
                            .map(|h| {
                                (
                                    String::from(h.name),
                                    std::str::from_utf8(h.value).unwrap().into(),
                                )
                            })
                            .collect(),
                    );
                    trace!("-- parse header");

                    debug!(
                        "Got HTTP request method {:#?} with body {} ",
                        headers_map, body
                    );

                    let cb_option = cb_clone.write().await;
                    trace!("-> callback");
                    if let Some(ref mut cb) = cb_option.as_ref() {
                        let cb_data = json!({ "headers": headers_map, "body": body,"method": req.method, "path": req.path, "version": req.version});
                        cb.on_data(cb_data.to_string());
                    }
                    let response = b"HTTP/1.1 200 OK\r\n\r\n";
                    tx.write_all(response).await.unwrap();
                    tx.flush().await.unwrap();
                }
                Err(e) => {
                    error!("couldn't get client: {:?}", e)
                }
            }
        }
        });
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OwnedTorService, TorHiddenServiceParam, TorServiceParam};
    use logger::Logger;

    use std::convert::TryInto;
    use std::sync::{Arc, Mutex};

    #[test]
    fn hidden_service_handler() {
        Logger::new();
        let socks_port = 19054;
        let mut owned_node: OwnedTorService = TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: String::from("/tmp/sifir_rs_sdk"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port: 20000,
                hs_port: 20011,
                secret_key: None,
            })
            .unwrap();
        assert!(service_key.onion_url.to_string().contains(".onion"));

        #[derive(Clone)]
        struct Observer {
            pub count: Arc<Mutex<u16>>,
            // FIXME make this into a string and store data for assertions
            // pub response: RefCel<String>,
        }
        impl DataObserver for Observer {
            fn on_data(&self, data: String) {
                let mut count = self.count.lock().unwrap();
                *count += 1;
                println!("Got data call number {} with {} ", count, data);
            }
            fn on_error(&self, data: String) {
                if data != "EOF" {
                    panic!("Got error!: {}", data);
                }
            }
        }
        let count = Arc::new(Mutex::new(0));
        let obv = Observer {
            count: count.clone(),
        };

        let mut listner = HiddenServiceHandler::new(20000).unwrap();
        let _ = listner.set_data_handler(obv.clone()).unwrap();
        let _ = listner.start_http_listener();

        ensure_runtime().lock().unwrap().block_on(async move {
            let client = utils::get_proxied_client(socks_port).unwrap();
            let mut onion_url =
                utils::reqwest::Url::parse(&format!("http://{}/my/path", service_key.onion_url))
                    .unwrap();
            let _ = onion_url.set_port(Some(20011));

            let resp = client
                .post(onion_url)
                .header("authorization", "secret-key")
                .body("secret p2p message")
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status().as_u16(), 200);
        });

        // Accessing Arc Mutex value for testing
        let count_value = *obv.count.lock().unwrap();
        assert_eq!(count_value, 1);

        owned_node.shutdown().unwrap();
    }
}
