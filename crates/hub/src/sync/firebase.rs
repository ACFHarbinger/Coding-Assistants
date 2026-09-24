//! Firebase Storage adapter (S10 / #100).
//!
//! Talks the Firebase Storage REST API through a [`Transport`] so unit tests
//! never hit the network. File names are BlobIds only. Credentials are not
//! in `Debug` or error text. Reserved identity/key-envelope recovery path
//! stays unused in v1.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::google_http::Transport;
use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use serde::Deserialize;
use std::fmt;

const STORAGE_URL: &str = "https://firebasestorage.googleapis.com/v0";

pub struct FirebaseStorage<T> {
    transport: T,
    bucket: String,
}

impl<T> fmt::Debug for FirebaseStorage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FirebaseStorage(***)")
    }
}

impl<T: Transport> FirebaseStorage<T> {
    pub fn new(transport: T, bucket: &str) -> Result<Self, SyncError> {
        Ok(Self {
            transport,
            bucket: parse_bucket(bucket)?,
        })
    }

    fn object_name(prefix: &RemotePrefix, id: &BlobId) -> String {
        format!("{}{}", prefix.as_str(), id.as_str())
    }

    fn list_url(&self, prefix: &str) -> String {
        format!(
            "{STORAGE_URL}/b/{}/o?prefix={}",
            self.bucket,
            url_encode(prefix)
        )
    }

    fn object_url(&self, name: &str, alt_media: bool) -> String {
        let mut url = format!("{STORAGE_URL}/b/{}/o/{}", self.bucket, url_encode(name));
        if alt_media {
            url.push_str("?alt=media");
        }
        url
    }

    fn upload_url(&self, name: &str) -> String {
        format!(
            "{STORAGE_URL}/b/{}/o?name={}",
            self.bucket,
            url_encode(name)
        )
    }

    fn lookup(&self, name: &str) -> Result<StorageObject, SyncError> {
        let response = self
            .transport
            .send("GET", &self.object_url(name, false), &[], None)?;
        status_to_error(response.status)?;
        parse_object(&response.body)
    }

    fn list_objects(&self, prefix: &str) -> Result<Vec<StorageObject>, SyncError> {
        let response = self
            .transport
            .send("GET", &self.list_url(prefix), &[], None)?;
        status_to_error(response.status)?;
        parse_object_list(&response.body)
    }

    fn upload(&self, name: &str, bytes: &[u8], generation: &str) -> Result<ETag, SyncError> {
        let response = self.transport.send(
            "POST",
            &self.upload_url(name),
            &[
                ("Content-Type", "application/octet-stream"),
                ("x-goog-if-generation-match", generation),
            ],
            Some(bytes),
        )?;
        status_to_error(response.status)?;
        Ok(parse_object(&response.body)?.object.etag)
    }

    fn find(&self, id: &BlobId) -> Result<StorageObject, SyncError> {
        self.list_objects("")?
            .into_iter()
            .find(|item| item.object.blob_id == *id)
            .ok_or(SyncError::NotFound)
    }
}

struct StorageObject {
    name: String,
    object: RemoteObject,
}

#[derive(Deserialize)]
struct ListPayload {
    #[serde(default)]
    items: Vec<ItemPayload>,
}

#[derive(Deserialize)]
struct ItemPayload {
    name: String,
    #[serde(default)]
    generation: Option<serde_json::Value>,
    #[serde(default)]
    size: Option<serde_json::Value>,
}

fn parse_bucket(bucket: &str) -> Result<String, SyncError> {
    let bucket = bucket.trim();
    if bucket.is_empty()
        || bucket.contains("..")
        || bucket.contains('/')
        || bucket.contains('\\')
        || !bucket
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return Err(SyncError::Invalid(
            "firebase storage bucket is invalid".into(),
        ));
    }
    Ok(bucket.to_string())
}

fn json_token(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn json_size(value: Option<&serde_json::Value>) -> u64 {
    match value {
        Some(serde_json::Value::String(text)) => text.parse().unwrap_or(0),
        Some(serde_json::Value::Number(number)) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn storage_object(item: ItemPayload) -> Option<StorageObject> {
    let segment = item.name.rsplit('/').next()?;
    let blob_id = BlobId::parse(segment).ok()?;
    Some(StorageObject {
        object: RemoteObject {
            blob_id,
            etag: ETag::new(json_token(item.generation.as_ref())),
            size: json_size(item.size.as_ref()),
        },
        name: item.name,
    })
}

fn parse_object_list(body: &[u8]) -> Result<Vec<StorageObject>, SyncError> {
    let parsed: ListPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("firebase storage list is not json".into()))?;
    Ok(parsed
        .items
        .into_iter()
        .filter_map(storage_object)
        .collect())
}

fn parse_object(body: &[u8]) -> Result<StorageObject, SyncError> {
    let item: ItemPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("firebase storage object is not json".into()))?;
    storage_object(item).ok_or_else(|| SyncError::Invalid("firebase storage object name".into()))
}

fn status_to_error(status: u16) -> Result<(), SyncError> {
    match status {
        200 | 201 | 204 => Ok(()),
        404 => Err(SyncError::NotFound),
        412 => Err(SyncError::Precondition),
        _ => Err(SyncError::Invalid("firebase storage request failed".into())),
    }
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

impl<T: Transport> DriveClient for FirebaseStorage<T> {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        Ok(self
            .list_objects(prefix.as_str())?
            .into_iter()
            .map(|item| item.object)
            .collect())
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        let found = self.find(id)?;
        let response =
            self.transport
                .send("GET", &self.object_url(&found.name, true), &[], None)?;
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
        let existing = match self.lookup(&name) {
            Ok(item) => Some(item),
            Err(SyncError::NotFound) => None,
            Err(error) => return Err(error),
        };
        match (existing, expected) {
            (None, None) => self.upload(&name, bytes, "0"),
            (Some(item), Some(etag)) if item.object.etag == *etag => {
                self.upload(&name, bytes, etag.as_str())
            }
            _ => Err(SyncError::Precondition),
        }
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let found = self.find(id)?;
        if found.object.etag != *etag {
            return Err(SyncError::Precondition);
        }
        let generation = etag.as_str().to_string();
        let response = self.transport.send(
            "DELETE",
            &self.object_url(&found.name, false),
            &[("x-goog-if-generation-match", generation.as_str())],
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
#[path = "firebase_tests.rs"]
mod tests;
