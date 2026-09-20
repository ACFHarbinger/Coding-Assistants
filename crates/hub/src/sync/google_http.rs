//! HTTP transport for the Google Drive adapter (S3 / #93).
//!
//! Production traffic uses ureq. Tests inject [`ScriptedTransport`] and never
//! hit the network. Authorization is applied here and must not appear in
//! `Debug`, recorded header names, or error text.

use super::SyncError;
use crate::secret::SecretString;
use std::fmt;
#[cfg(test)]
use std::sync::Mutex;
use std::time::Duration;
use ureq::RequestBuilder;

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub etag: Option<String>,
    pub body: Vec<u8>,
}

pub trait Transport {
    fn send(
        &self,
        method: &str,
        url: &str,
        extra_headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, SyncError>;
}

/// Production ureq transport. Authorization is applied here, never logged.
pub struct UreqTransport {
    agent: ureq::Agent,
    access_token: SecretString,
}

impl fmt::Debug for UreqTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UreqTransport(***)")
    }
}

impl UreqTransport {
    pub fn new(access_token: SecretString) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            access_token,
        }
    }
}

fn authorize<B>(
    mut request: RequestBuilder<B>,
    bearer: &str,
    extra_headers: &[(&str, &str)],
) -> RequestBuilder<B> {
    request = request.header("Authorization", bearer);
    for (name, value) in extra_headers {
        request = request.header(*name, *value);
    }
    request
}

fn map_result(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<HttpResponse, SyncError> {
    match result {
        Ok(mut response) => {
            let etag = response
                .headers()
                .get("etag")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let status = response.status().as_u16();
            let body = response
                .body_mut()
                .read_to_vec()
                .map_err(|_| SyncError::Invalid("google drive body read failed".into()))?;
            Ok(HttpResponse { status, etag, body })
        }
        Err(ureq::Error::StatusCode(status)) => Ok(HttpResponse {
            status,
            etag: None,
            body: Vec::new(),
        }),
        Err(_) => Err(SyncError::Invalid("google drive request failed".into())),
    }
}

impl Transport for UreqTransport {
    fn send(
        &self,
        method: &str,
        url: &str,
        extra_headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, SyncError> {
        let bearer = format!("Bearer {}", self.access_token.expose());
        match method {
            "GET" => map_result(authorize(self.agent.get(url), &bearer, extra_headers).call()),
            "DELETE" => {
                map_result(authorize(self.agent.delete(url), &bearer, extra_headers).call())
            }
            "POST" => {
                let request = authorize(self.agent.post(url), &bearer, extra_headers);
                map_result(match body {
                    Some(bytes) => request.send(bytes),
                    None => request.send_empty(),
                })
            }
            "PATCH" => {
                let request = authorize(self.agent.patch(url), &bearer, extra_headers);
                map_result(match body {
                    Some(bytes) => request.send(bytes),
                    None => request.send_empty(),
                })
            }
            _ => Err(SyncError::Invalid("google drive request failed".into())),
        }
    }
}

/// Scripted transport for tests. Records method/url/header names, never tokens.
#[cfg(test)]
pub struct ScriptedTransport {
    responses: Mutex<Vec<HttpResponse>>,
    pub recorded: Mutex<Vec<String>>,
}

#[cfg(test)]
impl ScriptedTransport {
    pub fn new(responses: Vec<HttpResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
            recorded: Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl Transport for ScriptedTransport {
    fn send(
        &self,
        method: &str,
        url: &str,
        extra_headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, SyncError> {
        let names: Vec<&str> = extra_headers.iter().map(|(name, _)| *name).collect();
        let body_text = body
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or("");
        self.recorded
            .lock()
            .unwrap()
            .push(format!("{method} {url} headers={names:?} body={body_text}"));
        let mut queue = self.responses.lock().unwrap();
        if queue.is_empty() {
            return Err(SyncError::Invalid("google drive request failed".into()));
        }
        Ok(queue.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ureq_transport_debug_omits_token() {
        let transport = UreqTransport::new(SecretString::new("ya29.secret-access"));
        let text = format!("{transport:?}");
        assert_eq!(text, "UreqTransport(***)");
        assert!(!text.contains("ya29"));
        assert!(!text.contains("secret-access"));
        assert!(!text.contains("Bearer"));
    }
}
