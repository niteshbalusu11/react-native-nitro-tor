use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapStatus {
    pub progress: u8,
    pub tag: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkLiveness {
    Up,
    Down,
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StatusParseError {
    #[error("invalid bootstrap status")]
    InvalidBootstrapStatus,
}

pub fn parse_bootstrap_status(input: &str) -> Result<BootstrapStatus, StatusParseError> {
    let input = input
        .trim()
        .strip_prefix("status/bootstrap-phase=")
        .unwrap_or(input.trim());
    let fields_start = input
        .find("PROGRESS=")
        .ok_or(StatusParseError::InvalidBootstrapStatus)?;
    let fields = parse_control_fields(&input[fields_start..])?;
    let progress = fields
        .get("PROGRESS")
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|progress| *progress <= 100)
        .ok_or(StatusParseError::InvalidBootstrapStatus)?;
    let tag = fields
        .get("TAG")
        .cloned()
        .ok_or(StatusParseError::InvalidBootstrapStatus)?;
    let summary = fields
        .get("SUMMARY")
        .cloned()
        .ok_or(StatusParseError::InvalidBootstrapStatus)?;

    Ok(BootstrapStatus {
        progress,
        tag,
        summary,
        warning: fields.get("WARNING").cloned(),
    })
}

pub fn parse_network_liveness(input: &str) -> NetworkLiveness {
    match input.trim().to_ascii_lowercase().as_str() {
        "up" => NetworkLiveness::Up,
        "down" => NetworkLiveness::Down,
        _ => NetworkLiveness::Unknown,
    }
}

pub fn parse_circuit_established(input: &str) -> Option<bool> {
    match input.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

fn parse_control_fields(input: &str) -> Result<HashMap<String, String>, StatusParseError> {
    let bytes = input.as_bytes();
    let mut fields = HashMap::new();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }

        let key_start = index;
        while index < bytes.len() && bytes[index] != b'=' && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == key_start || index == bytes.len() || bytes[index] != b'=' {
            return Err(StatusParseError::InvalidBootstrapStatus);
        }
        let key = &input[key_start..index];
        index += 1;

        let value = if index < bytes.len() && bytes[index] == b'"' {
            index += 1;
            let mut value = String::new();
            let mut closed = false;
            while index < bytes.len() {
                match bytes[index] {
                    b'"' => {
                        index += 1;
                        closed = true;
                        break;
                    }
                    b'\\' if index + 1 < bytes.len() => {
                        index += 1;
                        value.push(bytes[index] as char);
                        index += 1;
                    }
                    byte => {
                        value.push(byte as char);
                        index += 1;
                    }
                }
            }
            if !closed {
                return Err(StatusParseError::InvalidBootstrapStatus);
            }
            value
        } else {
            let value_start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            input[value_start..index].to_string()
        };

        fields.insert(key.to_string(), value);
    }

    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bootstrap_progress_snapshot() {
        let status = parse_bootstrap_status(
            "NOTICE BOOTSTRAP PROGRESS=80 TAG=ap_conn SUMMARY=\"Connecting to the Tor network\"",
        )
        .unwrap();

        assert_eq!(status.progress, 80);
        assert_eq!(status.tag, "ap_conn");
        assert_eq!(status.summary, "Connecting to the Tor network");
        assert_eq!(status.warning, None);
    }

    #[test]
    fn parses_bootstrap_warning_with_spaces() {
        let status = parse_bootstrap_status(
            "WARN BOOTSTRAP PROGRESS=10 TAG=conn SUMMARY=\"Connecting to a relay\" WARNING=\"Connection timed out\" REASON=TIMEOUT",
        )
        .unwrap();

        assert_eq!(status.progress, 10);
        assert_eq!(status.warning.as_deref(), Some("Connection timed out"));
    }

    #[test]
    fn rejects_bootstrap_status_without_required_fields() {
        assert!(parse_bootstrap_status("NOTICE BOOTSTRAP TAG=starting").is_err());
    }

    #[test]
    fn parses_connectivity_values() {
        assert_eq!(parse_network_liveness("up"), NetworkLiveness::Up);
        assert_eq!(parse_network_liveness("DOWN"), NetworkLiveness::Down);
        assert_eq!(
            parse_network_liveness("unexpected"),
            NetworkLiveness::Unknown
        );
        assert_eq!(parse_circuit_established("1"), Some(true));
        assert_eq!(parse_circuit_established("0"), Some(false));
        assert_eq!(parse_circuit_established("unknown"), None);
    }
}
