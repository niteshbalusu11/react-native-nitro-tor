use crate::TorErrors;
use crate::status::{
    BootstrapStatus, NetworkLiveness, parse_bootstrap_status, parse_network_liveness,
};
use std::fs;
use tokio::net::TcpStream;
use torut::control::{Conn, ConnError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TorControlEvent {
    Bootstrap(BootstrapStatus),
    NetworkLiveness(NetworkLiveness),
    CircuitChanged,
    CircuitEstablished(bool),
    ControlConnectionFailed(String),
}

pub fn parse_control_event(lines: &[String]) -> Option<TorControlEvent> {
    let line = lines.first()?.trim();
    if let Some(bootstrap) = line.strip_prefix("STATUS_CLIENT ") {
        return parse_bootstrap_status(bootstrap)
            .ok()
            .map(TorControlEvent::Bootstrap);
    }
    if let Some(liveness) = line.strip_prefix("NETWORK_LIVENESS ") {
        return Some(TorControlEvent::NetworkLiveness(parse_network_liveness(
            liveness,
        )));
    }
    if line.starts_with("CIRC ") {
        return Some(TorControlEvent::CircuitChanged);
    }
    None
}

pub fn cookie_path_from_protocol_info(lines: &[String]) -> Result<String, TorErrors> {
    let auth_line = lines
        .iter()
        .find(|line| line.starts_with("AUTH "))
        .ok_or_else(|| TorErrors::BootStrapError("Missing control authentication info".into()))?;
    let marker = "COOKIEFILE=";
    let start = auth_line
        .find(marker)
        .map(|index| index + marker.len())
        .ok_or_else(|| TorErrors::BootStrapError("Missing control cookie path".into()))?;
    let encoded = auth_line[start..].trim();
    if !encoded.starts_with('"') {
        return Ok(encoded
            .split_whitespace()
            .next()
            .unwrap_or(encoded)
            .to_string());
    }

    let mut path = String::new();
    let mut escaped = false;
    for character in encoded[1..].chars() {
        if escaped {
            path.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok(path);
        } else {
            path.push(character);
        }
    }
    Err(TorErrors::BootStrapError(
        "Invalid control cookie path".into(),
    ))
}

pub struct ControlConnection {
    conn: Conn<TcpStream>,
}

impl ControlConnection {
    pub async fn connect(address: &str) -> Result<Self, TorErrors> {
        let stream = TcpStream::connect(address.trim()).await?;
        let mut conn = Conn::new(stream);
        conn.write_data(b"PROTOCOLINFO 1\r\n").await?;
        let (code, lines) = conn.receive_data().await?;
        if code != 250 {
            return Err(TorErrors::ControlConnectionError(
                ConnError::InvalidResponseCode(code),
            ));
        }

        let cookie_path = cookie_path_from_protocol_info(&lines)?;
        let cookie = fs::read(cookie_path)?;
        conn.write_data(format!("AUTHENTICATE {}\r\n", hex::encode_upper(cookie)).as_bytes())
            .await?;
        let (code, _) = conn.receive_data().await?;
        if code != 250 {
            return Err(TorErrors::ControlConnectionError(
                ConnError::InvalidResponseCode(code),
            ));
        }
        Ok(Self { conn })
    }

    pub async fn command(&mut self, command: &str) -> Result<Vec<String>, TorErrors> {
        self.conn
            .write_data(format!("{}\r\n", command.trim()).as_bytes())
            .await?;
        loop {
            let (code, lines) = self.conn.receive_data().await?;
            if code == 650 {
                continue;
            }
            if code != 250 {
                return Err(TorErrors::ControlConnectionError(
                    ConnError::InvalidResponseCode(code),
                ));
            }
            return Ok(lines);
        }
    }

    pub async fn get_info(&mut self, key: &str) -> Result<String, TorErrors> {
        let lines = self.command(&format!("GETINFO {key}")).await?;
        let prefix = format!("{key}=");
        lines
            .into_iter()
            .find_map(|line| line.strip_prefix(&prefix).map(ToOwned::to_owned))
            .ok_or_else(|| TorErrors::BootStrapError(format!("Missing GETINFO response for {key}")))
    }

    pub async fn subscribe(&mut self) -> Result<(), TorErrors> {
        self.command("SETEVENTS STATUS_CLIENT NETWORK_LIVENESS CIRC")
            .await?;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<TorControlEvent, TorErrors> {
        loop {
            let (code, lines) = self.conn.receive_data().await?;
            if code == 650 {
                if let Some(event) = parse_control_event(&lines) {
                    return Ok(event);
                }
            }
        }
    }

    pub async fn request_new_identity(&mut self) -> Result<(), TorErrors> {
        self.command("SIGNAL NEWNYM").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_cookie_path_from_protocol_info() {
        let lines = vec![
            "PROTOCOLINFO 1".to_string(),
            "AUTH METHODS=COOKIE,SAFECOOKIE COOKIEFILE=\"/tmp/tor data/control_auth_cookie\""
                .to_string(),
            "VERSION Tor=\"0.4.9.11\"".to_string(),
            "OK".to_string(),
        ];

        assert_eq!(
            cookie_path_from_protocol_info(&lines).unwrap(),
            "/tmp/tor data/control_auth_cookie"
        );
    }

    #[test]
    fn parses_status_and_liveness_events() {
        let bootstrap = parse_control_event(&[
            "STATUS_CLIENT NOTICE BOOTSTRAP PROGRESS=45 TAG=requesting_descriptors SUMMARY=\"Requesting relay descriptors\""
                .to_string(),
        ])
        .unwrap();
        assert!(matches!(
            bootstrap,
            TorControlEvent::Bootstrap(BootstrapStatus { progress: 45, .. })
        ));

        assert_eq!(
            parse_control_event(&["NETWORK_LIVENESS DOWN".to_string()]),
            Some(TorControlEvent::NetworkLiveness(NetworkLiveness::Down))
        );
    }

    #[test]
    fn parses_circuit_events() {
        assert_eq!(
            parse_control_event(&["CIRC 42 BUILT PURPOSE=GENERAL".to_string()]),
            Some(TorControlEvent::CircuitChanged)
        );
        assert_eq!(
            parse_control_event(&["STREAM 7 NEW 0 example.com:443".to_string()]),
            None
        );
    }
}
