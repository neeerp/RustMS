//! Extract string literals from Lua sources, in order, for the content
//! importer. Only what the community scripts use is supported: quoted
//! strings with the standard escapes, long strings, and comments.

/// Every string literal in `source`, decoded, in source order.
pub fn string_literals(source: &str) -> Result<Vec<String>, String> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                i += 2;
                if let Some(level) = long_bracket_level(bytes, i) {
                    i = skip_long(bytes, i, level)?.1;
                } else {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
            }
            b'"' | b'\'' => {
                let (text, next) = quoted(bytes, i)?;
                out.push(text);
                i = next;
            }
            b'[' => match long_bracket_level(bytes, i) {
                Some(level) => {
                    let (text, next) = skip_long(bytes, i, level)?;
                    out.push(text);
                    i = next;
                }
                None => i += 1,
            },
            _ => i += 1,
        }
    }
    Ok(out)
}

/// `[[`, `[=[`, ... at `i`: the number of `=`.
fn long_bracket_level(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let mut level = 0;
    while bytes.get(i + 1 + level) == Some(&b'=') {
        level += 1;
    }
    (bytes.get(i + 1 + level) == Some(&b'[')).then_some(level)
}

fn skip_long(bytes: &[u8], start: usize, level: usize) -> Result<(String, usize), String> {
    let open = start + level + 2;
    let close: Vec<u8> = std::iter::once(b']')
        .chain(std::iter::repeat_n(b'=', level))
        .chain(std::iter::once(b']'))
        .collect();
    let end = bytes[open..]
        .windows(close.len())
        .position(|w| w == close.as_slice())
        .ok_or("unterminated long string")?;
    // A newline right after the opening bracket is not part of the string.
    let mut body = &bytes[open..open + end];
    if body.first() == Some(&b'\n') {
        body = &body[1..];
    }
    let text = String::from_utf8(body.to_vec()).map_err(|e| e.to_string())?;
    Ok((text, open + end + close.len()))
}

fn quoted(bytes: &[u8], start: usize) -> Result<(String, usize), String> {
    let quote = bytes[start];
    let mut out = Vec::new();
    let mut i = start + 1;
    loop {
        let b = *bytes.get(i).ok_or("unterminated string")?;
        i += 1;
        match b {
            b'\n' => return Err("newline in string".into()),
            b if b == quote => break,
            b'\\' => {
                let e = *bytes.get(i).ok_or("unterminated escape")?;
                i += 1;
                match e {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'a' => out.push(0x07),
                    b'b' => out.push(0x08),
                    b'f' => out.push(0x0c),
                    b'v' => out.push(0x0b),
                    b'\\' | b'"' | b'\'' | b'\n' => out.push(e),
                    b'0'..=b'9' => {
                        let mut value = u32::from(e - b'0');
                        for _ in 0..2 {
                            match bytes.get(i) {
                                Some(d @ b'0'..=b'9') => {
                                    value = value * 10 + u32::from(d - b'0');
                                    i += 1;
                                }
                                _ => break,
                            }
                        }
                        out.push(u8::try_from(value).map_err(|_| "escape out of range")?);
                    }
                    other => return Err(format!("unsupported escape \\{}", other as char)),
                }
            }
            b => out.push(b),
        }
    }
    Ok((String::from_utf8(out).map_err(|e| e.to_string())?, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quoted_and_long_strings_in_order() {
        let source = r#"
-- a comment with "quotes"
--[[ a long
comment with 'quotes' ]]
Self.Say("Hey, " .. (g == 0 and "Man" or 'Miss') .. "~ What's up?\r\nNext")
x = [==[
long ]] string]==]
"#;
        assert_eq!(
            string_literals(source).unwrap(),
            [
                "Hey, ",
                "Man",
                "Miss",
                "~ What's up?\r\nNext",
                "long ]] string"
            ]
        );
    }

    #[test]
    fn decodes_escapes() {
        assert_eq!(
            string_literals(r#"'it\'s' "a\\b" "\65\066""#).unwrap(),
            ["it's", "a\\b", "AB"]
        );
        assert!(string_literals("\"open").is_err());
    }
}
