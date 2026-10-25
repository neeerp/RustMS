//! The vote hook: a vote site calls it when a player votes for the server,
//! and the player's account gets NX Credit (`db::vote`). It is ours, not
//! v83's, and off unless `RUSTMS_VOTE_HOOK_ADDR` and
//! `RUSTMS_VOTE_HOOK_SECRET` are both set.
//!
//! A site sends `GET` or `POST /vote` with `account` (the login name) and
//! `secret` in the query or a form body; the secret may come in an
//! `X-Vote-Secret` header instead. The answer is plain text: 200 for a
//! counted vote, 429 for one within the cooldown, 404 for no such account
//! (or path), 403 for a wrong secret, 405 for another method, 400 for a
//! request it can't read and 500 if the database fails.

use crate::db::spawn_db;
use db::vote::Vote;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// The most of a request read: a site's call is a line and a few headers.
const MAX_HEAD: usize = 8 * 1024;
const MAX_BODY: usize = 4 * 1024;
/// How long a caller has to send its request, and to take the answer.
const TIMEOUT: Duration = Duration::from_secs(5);
/// The most calls answered at once; more are closed unanswered, so idle
/// sockets can't use up the world's file descriptors.
const MAX_CALLS: usize = 64;
/// The longest cooldown: a century.
const MAX_COOLDOWN_HOURS: i64 = 24 * 365 * 100;

pub struct VoteHook {
    addr: SocketAddr,
    secret: String,
    /// NX Credit a vote gives: `RUSTMS_VOTE_NX`, 1000 when unset.
    amount: i32,
    /// How long after a vote the account may vote again:
    /// `RUSTMS_VOTE_COOLDOWN_HOURS`, 12 when unset.
    cooldown: Duration,
}

impl VoteHook {
    /// The hook as the environment sets it up, if it does.
    pub fn from_env() -> Option<Self> {
        let addr = std::env::var("RUSTMS_VOTE_HOOK_ADDR").ok()?;
        let Ok(addr) = addr.parse() else {
            warn!(
                addr,
                "RUSTMS_VOTE_HOOK_ADDR is no address; the vote hook is off"
            );
            return None;
        };
        let secret = std::env::var("RUSTMS_VOTE_HOOK_SECRET")
            .unwrap_or_default()
            .trim()
            .to_string();
        if secret.is_empty() {
            warn!("RUSTMS_VOTE_HOOK_SECRET is unset; the vote hook is off");
            return None;
        }
        let number = |key: &str, default: i64| {
            std::env::var(key)
                .ok()
                .and_then(|value| value.trim().parse::<i64>().ok())
                .filter(|&value| value > 0)
                .unwrap_or(default)
        };
        Some(VoteHook {
            addr,
            secret,
            amount: number("RUSTMS_VOTE_NX", 1000).min(i64::from(i32::MAX)) as i32,
            cooldown: Duration::from_secs(
                number("RUSTMS_VOTE_COOLDOWN_HOURS", 12).min(MAX_COOLDOWN_HOURS) as u64 * 3600,
            ),
        })
    }

    /// Answer calls until the listener fails.
    pub async fn serve(self) {
        let listener = match TcpListener::bind(self.addr).await {
            Ok(listener) => listener,
            Err(error) => {
                error!(addr = %self.addr, %error, "Vote hook failed to listen");
                return;
            }
        };
        info!(addr = %self.addr, amount = self.amount, "Vote hook listening");
        let hook = Arc::new(self);
        let calls = Arc::new(Semaphore::new(MAX_CALLS));
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let Ok(call) = calls.clone().try_acquire_owned() else {
                        continue;
                    };
                    let hook = hook.clone();
                    tokio::spawn(async move {
                        if let Err(error) = hook.handle(stream, peer).await {
                            warn!(%peer, %error, "Vote hook call failed");
                        }
                        drop(call);
                    });
                }
                // Such as running out of file descriptors: wait, and try
                // again.
                Err(error) => {
                    warn!(%error, "Vote hook failed to accept a call");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn handle(&self, mut stream: TcpStream, peer: SocketAddr) -> std::io::Result<()> {
        let request = tokio::time::timeout(TIMEOUT, read_request(&mut stream))
            .await
            .unwrap_or(Ok(None))?;
        let (status, body) = match request {
            Some(request) => self.answer(request, peer).await,
            None => (400, "bad request".to_string()),
        };
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            429 => "Too Many Requests",
            _ => "Internal Server Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let write = async {
            stream.write_all(response.as_bytes()).await?;
            stream.shutdown().await
        };
        tokio::time::timeout(TIMEOUT, write).await.unwrap_or(Ok(()))
    }

    async fn answer(&self, request: Request, peer: SocketAddr) -> (u16, String) {
        if request.path != "/vote" {
            return (404, "not found".into());
        }
        if request.method != "GET" && request.method != "POST" {
            return (405, "use GET or POST".into());
        }
        let secret = request
            .params
            .get("secret")
            .map(String::as_str)
            .unwrap_or("");
        if !same(secret.as_bytes(), self.secret.as_bytes()) {
            warn!(%peer, "Vote hook call with a wrong secret");
            return (403, "wrong secret".into());
        }
        let Some(account) = request
            .params
            .get("account")
            .filter(|a| !a.is_empty())
            .cloned()
        else {
            return (400, "no account".into());
        };
        let (amount, cooldown) = (self.amount, self.cooldown);
        let name = account.clone();
        let vote = spawn_db(move || db::vote::record(&name, amount, cooldown, SystemTime::now()));
        match vote.await {
            Ok(Vote::Counted(balance)) => {
                info!(account, amount, credit = balance.credit, "Vote counted");
                (200, "counted".into())
            }
            Ok(Vote::TooSoon(next)) => {
                let wait = next
                    .duration_since(SystemTime::now())
                    .unwrap_or_default()
                    .as_secs();
                (429, format!("too soon; vote again in {wait} seconds"))
            }
            Ok(Vote::NoAccount) => (404, "no such account".into()),
            Err(error) => {
                error!(account, %error, "Failed to count a vote");
                (500, "error".into())
            }
        }
    }
}

/// Whether `a` and `b` are equal, taking as long whichever byte differs.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[derive(Debug, PartialEq, Eq)]
struct Request {
    method: String,
    path: String,
    /// The query's and the form body's fields, and `secret` from the
    /// header if neither has it.
    params: HashMap<String, String>,
}

/// Read a request; `None` if it is malformed or too big.
async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(at) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        if buffer.len() > MAX_HEAD {
            return Ok(None);
        }
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let Ok(head) = std::str::from_utf8(&buffer[..head_end]) else {
        return Ok(None);
    };
    let length = content_length(head).unwrap_or(0);
    if length > MAX_BODY {
        return Ok(None);
    }
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    Ok(parse(head, &body))
}

fn headers(head: &str) -> impl Iterator<Item = (&str, &str)> {
    head.split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
}

fn content_length(head: &str) -> Option<usize> {
    headers(head)
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
}

fn parse(head: &str, body: &[u8]) -> Option<Request> {
    let mut line = head.split("\r\n").next()?.split(' ');
    let (method, target) = (line.next()?, line.next()?);
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut params = HashMap::new();
    fields(query, &mut params)?;
    let form = headers(head).any(|(name, value)| {
        name.eq_ignore_ascii_case("content-type")
            && value.starts_with("application/x-www-form-urlencoded")
    });
    if form {
        fields(std::str::from_utf8(body).ok()?, &mut params)?;
    }
    if let Some((_, secret)) =
        headers(head).find(|(name, _)| name.eq_ignore_ascii_case("x-vote-secret"))
    {
        params
            .entry("secret".into())
            .or_insert_with(|| secret.to_string());
    }
    Some(Request {
        method: method.to_string(),
        path: path.to_string(),
        params,
    })
}

/// Add the `a=b&c=d` fields of `form` to `params`; the first of a name
/// counts.
fn fields(form: &str, params: &mut HashMap<String, String>) -> Option<()> {
    for pair in form.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.entry(decode(name)?).or_insert(decode(value)?);
    }
    Some(())
}

/// A form field's text: `+` is a space and `%XX` a byte. `None` for a
/// malformed escape or a NUL, which no login name has.
fn decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        bytes.push(match byte {
            b'+' => b' ',
            b'%' => {
                let hex = [rest.next()?, rest.next()?];
                if !hex.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?
            }
            byte => byte,
        });
    }
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(request: &Request) -> Vec<(&str, &str)> {
        let mut params: Vec<_> = request
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        params.sort();
        params
    }

    #[test]
    fn fields_come_from_the_query_the_form_and_the_header() {
        let get = parse(
            "GET /vote?account=Ann%20B&secret=s+1 HTTP/1.1\r\nHost: x",
            b"",
        )
        .unwrap();
        assert_eq!((get.method.as_str(), get.path.as_str()), ("GET", "/vote"));
        assert_eq!(params(&get), [("account", "Ann B"), ("secret", "s 1")]);

        let post = parse(
            "POST /vote HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nX-Vote-Secret: hdr",
            b"account=ann&account=other",
        )
        .unwrap();
        assert_eq!(params(&post), [("account", "ann"), ("secret", "hdr")]);

        // A body that isn't a form is not read; a query's secret beats the
        // header's.
        let json = parse(
            "POST /vote?secret=q HTTP/1.1\r\nContent-Type: application/json\r\nx-vote-secret: hdr",
            b"account=ann",
        )
        .unwrap();
        assert_eq!(params(&json), [("secret", "q")]);

        assert_eq!(parse("GET /vote?account=%zz HTTP/1.1", b""), None);
        assert_eq!(parse("GET /vote?account=%4 HTTP/1.1", b""), None);
        assert_eq!(parse("GET /vote?account=%+f HTTP/1.1", b""), None);
        assert_eq!(parse("GET /vote?account=a%00 HTTP/1.1", b""), None);
        assert_eq!(parse("", b""), None);
    }

    #[test]
    fn content_length_is_read_whatever_its_case() {
        assert_eq!(
            content_length("POST / HTTP/1.1\r\ncontent-LENGTH: 12"),
            Some(12)
        );
        assert_eq!(content_length("GET / HTTP/1.1\r\nHost: x"), None);
    }

    #[test]
    fn secrets_must_match_exactly() {
        assert!(same(b"abc", b"abc"));
        assert!(!same(b"abc", b"abd"));
        assert!(!same(b"abc", b"abcd"));
        assert!(!same(b"", b"abc"));
    }
}
