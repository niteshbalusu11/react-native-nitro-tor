pub use reqwest;

pub fn get_proxied_client(socks_port: u16) -> Result<reqwest::Client, reqwest::Error> {
    let proxy = reqwest::Proxy::all(
        reqwest::Url::parse(format!("socks5h://127.0.0.1:{}", socks_port).as_str()).unwrap(),
    )
    .unwrap();
    reqwest::Client::builder().proxy(proxy).build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
