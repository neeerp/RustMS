/// Convert bytes to a hex String.
pub fn to_hex_string(bytes: Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join(" ")
}
