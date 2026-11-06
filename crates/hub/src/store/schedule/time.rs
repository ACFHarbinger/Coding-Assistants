//! UTC instant helpers for Scheduled Goals (#351).
//!
//! Stored timestamps are canonical RFC3339 UTC with millisecond precision
//! and a `Z` suffix so lexical order matches chronological order. Tick
//! `now` values are parsed as real instants, never compared as raw text.

use super::super::HubError;
use chrono::{DateTime, SecondsFormat, Utc};

pub fn parse_utc(label: &str, value: &str) -> Result<DateTime<Utc>, HubError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| HubError::Invalid(format!("{label} must be an RFC3339 timestamp")))
}

pub fn to_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn now_utc_rfc3339() -> String {
    to_rfc3339(Utc::now())
}

pub fn normalize_rfc3339(label: &str, value: &str) -> Result<String, HubError> {
    Ok(to_rfc3339(parse_utc(label, value)?))
}

pub fn seconds_between(earlier: &str, later: &str) -> Result<i64, HubError> {
    Ok((parse_utc("later", later)? - parse_utc("earlier", earlier)?).num_seconds())
}

pub fn add_seconds(ts: &str, secs: i64) -> Result<String, HubError> {
    Ok(to_rfc3339(
        parse_utc("timestamp", ts)? + chrono::Duration::seconds(secs),
    ))
}

pub fn instant_le(left: &str, right: &str) -> Result<bool, HubError> {
    Ok(parse_utc("left", left)? <= parse_utc("right", right)?)
}
