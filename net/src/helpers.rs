use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::NetworkError;

/// Convert bytes to a hex String.
pub fn to_hex_string(bytes: &Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join(" ")
}

pub fn current_time_i64() -> Result<i64, NetworkError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
}
