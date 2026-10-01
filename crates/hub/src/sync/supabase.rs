//! Private Supabase Storage adapter (S11).
//!
//! Object names are approved remote prefixes plus hashed BlobIds. The adapter
//! never sends a replica key, creates public URLs, or implements recovery.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::google_http::Transport;
use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use serde::Deserialize;
use std::fmt;

pub struct SupabaseStorage<T> {
    transport: T,
    origin: String,
    bucket: String,
}

impl<T> fmt::Debug for SupabaseStorage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SupabaseStorage(***)")
    }
}

impl<T: Transport> SupabaseStorage<T> {
    pub fn new(transport: T, origin: &str, bucket: &str) -> Result<Self, SyncError> {
        Ok(Self {
            transport,
            origin: super::supabase_auth::parse_project_url(origin)?,
            bucket: parse_bucket(bucket)?,
        })
    }

    fn object_name(prefix: &RemotePrefix, id: &BlobId) -> String {
        format!("{}{}", prefix.as_str(), id.as_str())
    }

    fn list_url(&self) -> String {
        format!("{}/storage/v1/object/list/{}", self.origin, self.bucket)
    }

    fn object_url(&self, name: &str) -> String {
        format!(
            "{}/storage/v1/object/{}/{}",
            self.origin,
            self.bucket,
            encode_object_path(name)
        )
    }

    fn list_objects(&self, prefix: &str) -> Result<Vec<StorageObject>, SyncError> {
        let body = serde_json::json!({"prefix": prefix}).to_string();
        let response = self.transport.send(
            "POST",
            &self.list_url(),
            &[("Content-Type", "application/json")],
            Some(body.as_bytes()),
        )?;
        status_to_error(response.status)?;
        parse_object_list(&response.body)
    }

    fn find_name(&self, name: &str) -> Result<StorageObject, SyncError> {
        self.list_objects(name)?
            .into_iter()
            .find(|item| item.name == name)
            .ok_or(SyncError::NotFound)
    }

    fn find(&self, id: &BlobId) -> Result<StorageObject, SyncError> {
        self.list_objects("")?
            .into_iter()
            .find(|item| item.object.blob_id == *id)
            .ok_or(SyncError::NotFound)
    }

    fn upload(&self, name: &str, bytes: &[u8], expected: Option<&ETag>) -> Result<ETag, SyncError> {
        let mut headers = vec![
            ("Content-Type", "application/octet-stream"),
            (
                "x-upsert",
                if expected.is_some() { "true" } else { "false" },
            ),
        ];
        if let Some(etag) = expected {
            headers.push(("If-Match", etag.as_str()));
        }
        let response =
            self.transport
                .send("POST", &self.object_url(name), &headers, Some(bytes))?;
        status_to_error(response.status)?;
        // Storage's object endpoint does not promise an ETag response header.
        // Re-list the exact safe name to obtain the server's opaque version.
        self.find_name(name).map(|object| object.object.etag)
    }
}

struct StorageObject {
    name: String,
    object: RemoteObject,
}

#[derive(Deserialize)]
struct ItemPayload {
    name: String,
    #[serde(default)]
    metadata: Metadata,
}

#[derive(Default, Deserialize)]
struct Metadata {
    #[serde(rename = "eTag", default)]
    etag: String,
    #[serde(default)]
    size: serde_json::Value,
}

fn parse_bucket(bucket: &str) -> Result<String, SyncError> {
    let bucket = bucket.trim();
    if bucket.is_empty()
        || bucket.contains("..")
        || bucket.contains(['/', '\\'])
        || !bucket
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(SyncError::Invalid(
            "supabase storage bucket is invalid".into(),
        ));
    }
    Ok(bucket.to_string())
}

fn json_size(value: &serde_json::Value) -> u64 {
    match value {
        serde_json::Value::String(text) => text.parse().unwrap_or(0),
        serde_json::Value::Number(number) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn storage_object(item: ItemPayload) -> Option<StorageObject> {
    let blob_id = BlobId::parse(item.name.rsplit('/').next()?).ok()?;
    if item.metadata.etag.is_empty() {
        return None;
    }
    Some(StorageObject {
        name: item.name,
        object: RemoteObject {
            blob_id,
            etag: ETag::new(item.metadata.etag),
            size: json_size(&item.metadata.size),
        },
    })
}

fn parse_object_list(body: &[u8]) -> Result<Vec<StorageObject>, SyncError> {
    let parsed: Vec<ItemPayload> = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("supabase storage list is not json".into()))?;
    Ok(parsed.into_iter().filter_map(storage_object).collect())
}

fn status_to_error(status: u16) -> Result<(), SyncError> {
    match status {
        200 | 201 | 204 => Ok(()),
        404 => Err(SyncError::NotFound),
        409 | 412 => Err(SyncError::Precondition),
        _ => Err(SyncError::Invalid("supabase storage request failed".into())),
    }
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Supabase object URLs are path segments (`replica/<id>`), not a single
/// percent-encoded name. Encoding `/` as `%2F` misses the live object.
fn encode_object_path(name: &str) -> String {
    name.split('/')
        .map(url_encode)
        .collect::<Vec<_>>()
        .join("/")
}

impl<T: Transport> DriveClient for SupabaseStorage<T> {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        Ok(self
            .list_objects(prefix.as_str())?
            .into_iter()
            .map(|item| item.object)
            .collect())
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        let found = self.find(id)?;
        let response = self
            .transport
            .send("GET", &self.object_url(&found.name), &[], None)?;
        status_to_error(response.status)?;
        Ok(response.body)
    }

    fn put_if_unmodified(
        &mut self,
        prefix: &RemotePrefix,
        id: &BlobId,
        bytes: &[u8],
        expected: Option<&ETag>,
    ) -> Result<ETag, SyncError> {
        let name = Self::object_name(prefix, id);
        let existing = match self.find_name(&name) {
            Ok(item) => Some(item),
            Err(SyncError::NotFound) => None,
            Err(error) => return Err(error),
        };
        match (existing, expected) {
            (None, None) => self.upload(&name, bytes, None),
            (Some(item), Some(etag)) if item.object.etag == *etag => {
                self.upload(&name, bytes, Some(etag))
            }
            _ => Err(SyncError::Precondition),
        }
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let found = self.find(id)?;
        if found.object.etag != *etag {
            return Err(SyncError::Precondition);
        }
        let response = self.transport.send(
            "DELETE",
            &self.object_url(&found.name),
            &[("If-Match", etag.as_str())],
            None,
        )?;
        status_to_error(response.status)
    }

    fn advance_replica(&mut self, plan: ReplicaAdvance) -> Result<(), SyncError> {
        for put in &plan.puts {
            self.put_if_unmodified(
                &put.prefix,
                &put.blob_id,
                &put.bytes,
                put.expected_etag.as_ref(),
            )?;
        }
        if let Some(prefix) = plan.prune_prefix {
            for object in self.list(&prefix)? {
                self.delete_if_match(&object.blob_id, &object.etag)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "supabase_tests.rs"]
mod tests;
