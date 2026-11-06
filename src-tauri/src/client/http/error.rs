//! Structured errors for direct HTTP model calls (`platform.md` P4).
//!
//! Messages are secret-free: they never include the API key, prompt, or
//! response body. Display text is what crosses into `LLMClient`'s `String`
//! error channel.

use async_openai::error::OpenAIError;
use std::fmt;

/// Failure class for an OpenAI-compatible `chat/completions` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectHttpError {
    Unauthenticated { hint: String },
    Transport { message: String },
    Timeout,
    Api { message: String },
    InvalidResponse { message: String },
    Cancelled,
}

impl DirectHttpError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated { .. } => "unauthenticated",
            Self::Transport { .. } => "transport",
            Self::Timeout => "timeout",
            Self::Api { .. } => "api",
            Self::InvalidResponse { .. } => "invalid_response",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn unavailable(hint: impl Into<String>) -> Self {
        Self::Unauthenticated { hint: hint.into() }
    }

    /// Whether a fallback model in a P13 gateway chain should be tried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport { .. }
                | Self::Timeout
                | Self::Api { .. }
                | Self::InvalidResponse { .. }
        )
    }
}

impl fmt::Display for DirectHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthenticated { hint } => write!(f, "{hint}"),
            Self::Transport { message } => {
                write!(f, "OpenAI-compatible request failed ({message})")
            }
            Self::Timeout => write!(
                f,
                "OpenAI-compatible request timed out. Check connectivity and retry."
            ),
            Self::Api { message } => write!(f, "OpenAI-compatible API error: {message}"),
            Self::InvalidResponse { message } => {
                write!(f, "OpenAI-compatible response was invalid ({message})")
            }
            Self::Cancelled => write!(f, "Task cancelled"),
        }
    }
}

impl std::error::Error for DirectHttpError {}

impl From<DirectHttpError> for String {
    fn from(error: DirectHttpError) -> Self {
        error.to_string()
    }
}

pub fn map_openai_error(error: OpenAIError) -> DirectHttpError {
    match error {
        OpenAIError::Reqwest(inner) => {
            if inner.is_timeout() {
                DirectHttpError::Timeout
            } else {
                DirectHttpError::Transport {
                    message: inner.to_string(),
                }
            }
        }
        OpenAIError::ApiError(api) => DirectHttpError::Api {
            message: api.message,
        },
        OpenAIError::JSONDeserialize(inner) => DirectHttpError::InvalidResponse {
            message: inner.to_string(),
        },
        OpenAIError::StreamError(message) => DirectHttpError::Transport { message },
        OpenAIError::InvalidArgument(message) => DirectHttpError::InvalidResponse { message },
        OpenAIError::FileSaveError(message) | OpenAIError::FileReadError(message) => {
            DirectHttpError::InvalidResponse { message }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable_and_display_is_secret_free() {
        let unauth = DirectHttpError::unavailable("OpenAI unavailable: not authenticated.");
        assert_eq!(unauth.code(), "unauthenticated");
        assert!(!unauth.to_string().contains("sk-"));

        let api = DirectHttpError::Api {
            message: "invalid_request_error".into(),
        };
        assert_eq!(api.code(), "api");
        assert!(api.to_string().contains("invalid_request_error"));
        assert_eq!(DirectHttpError::Timeout.code(), "timeout");
        assert_eq!(DirectHttpError::Cancelled.code(), "cancelled");
        assert!(DirectHttpError::Timeout.is_retryable());
        assert!(!DirectHttpError::Cancelled.is_retryable());
        assert!(!unauth.is_retryable());
    }

    #[test]
    fn string_conversion_matches_display() {
        let error = DirectHttpError::Timeout;
        let as_string: String = error.clone().into();
        assert_eq!(as_string, error.to_string());
    }
}
