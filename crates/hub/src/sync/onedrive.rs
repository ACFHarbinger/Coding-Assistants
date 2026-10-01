//! Private OneDrive App Folder adapter (S12).
//!
//! Graph `special/approot` only. Object names are approved prefixes plus
//! hashed BlobIds. Never creates sharing links or uploads the replica key.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::google_http::Transport;
use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use serde::Deserialize;
use std::fmt;

const GRAPH: &str = "https://graph.microsoft.com/v1.0";

pub struct OneDriveStorage<T> {
    transport: T,
}

impl<T> fmt::Debug for OneDriveStorage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OneDriveStorage(***)")
    }
}

impl<T: Transport> OneDriveStorage<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    fn object_name(prefix: &RemotePrefix, id: &BlobId) -> String {
        format!("{}{}", prefix.as_str(), id.as_str())
    }

    fn item_url(name: &str) -> String {
        format!(
            "{GRAPH}/me/drive/special/approot:/{}:",
            encode_object_path(name)
        )
    }

    fn content_url(name: &str) -> String {
        format!(
            "{GRAPH}/me/drive/special/approot:/{}:/content",
            encode_object_path(name)
        )
    }

    fn children_url(prefix: &str) -> String {
        let folder = prefix.trim_end_matches('/');
        if folder.is_empty() {
            format!("{GRAPH}/me/drive/special/approot/children")
        } else {
            format!(
                "{GRAPH}/me/drive/special/approot:/{}:/children",
                encode_object_path(folder)
            )
        }
    }

    fn lookup(&self, name: &str) -> Result<StorageObject, SyncError> {
        let response = self
            .transport
            .send("GET", &Self::item_url(name), &[], None)?;
        if response.status == 404 {
            return Err(SyncError::NotFound);
        }
        status_to_error(response.status)?;
        parse_object(&response.body, Some(name))
    }

    fn list_objects(&self, prefix: &str) -> Result<Vec<StorageObject>, SyncError> {
        let response = self
            .transport
            .send("GET", &Self::children_url(prefix), &[], None)?;
        if response.status == 404 {
            return Ok(Vec::new());
        }
        status_to_error(response.status)?;
        parse_object_list(&response.body, prefix)
    }

    fn upload(&self, name: &str, bytes: &[u8], expected: Option<&ETag>) -> Result<ETag, SyncError> {
        let if_header = match expected {
            Some(etag) => ("If-Match", etag.as_str()),
            None => ("If-None-Match", "*"),
        };
        let response = self.transport.send(
            "PUT",
            &Self::content_url(name),
            &[("Content-Type", "application/octet-stream"), if_header],
            Some(bytes),
        )?;
        status_to_error(response.status)?;
        parse_object(&response.body, Some(name)).map(|item| item.object.etag)
    }

    fn find(&self, id: &BlobId) -> Result<StorageObject, SyncError> {
        let url = format!(
            "{GRAPH}/me/drive/special/approot/search(q='{}')",
            id.as_str()
        );
        let response = self.transport.send("GET", &url, &[], None)?;
        if response.status == 404 {
            return Err(SyncError::NotFound);
        }
        status_to_error(response.status)?;
        parse_object_list(&response.body, "")?
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
    value: Vec<ItemPayload>,
}

#[derive(Deserialize)]
struct ItemPayload {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "eTag")]
    etag: String,
    #[serde(default)]
    size: serde_json::Value,
    #[serde(default, rename = "parentReference")]
    parent: Option<ParentRef>,
}

#[derive(Deserialize)]
struct ParentRef {
    #[serde(default)]
    path: String,
}

fn json_size(value: &serde_json::Value) -> u64 {
    match value {
        serde_json::Value::String(text) => text.parse().unwrap_or(0),
        serde_json::Value::Number(number) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn item_path(item: &ItemPayload, fallback: Option<&str>) -> Option<String> {
    if let Some(path) = fallback.filter(|value| !value.is_empty()) {
        return Some(path.to_string());
    }
    let blob = BlobId::parse(item.name.rsplit('/').next()?).ok()?;
    if let Some(parent) = item
        .parent
        .as_ref()
        .and_then(|row| relative_parent(&row.path))
    {
        return Some(format!("{parent}/{}", blob.as_str()));
    }
    Some(blob.as_str().to_string())
}

fn relative_parent(path: &str) -> Option<String> {
    let marker = "/approot:";
    let rest = path.split(marker).nth(1)?.trim_start_matches('/');
    if rest.is_empty() {
        None
    } else {
        Some(rest.trim_end_matches('/').to_string())
    }
}

fn storage_object(item: ItemPayload, fallback: Option<&str>) -> Option<StorageObject> {
    let name = item_path(&item, fallback)?;
    let blob_id = BlobId::parse(name.rsplit('/').next()?).ok()?;
    if item.etag.is_empty() {
        return None;
    }
    Some(StorageObject {
        name,
        object: RemoteObject {
            blob_id,
            etag: ETag::new(item.etag.trim_matches('"').to_string()),
            size: json_size(&item.size),
        },
    })
}

fn parse_object_list(body: &[u8], prefix: &str) -> Result<Vec<StorageObject>, SyncError> {
    let parsed: ListPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("onedrive list is not json".into()))?;
    Ok(parsed
        .value
        .into_iter()
        .filter_map(|item| {
            let filename = item.name.rsplit('/').next().unwrap_or("");
            let composed = if prefix.is_empty() {
                None
            } else {
                Some(format!("{prefix}{filename}"))
            };
            storage_object(item, composed.as_deref())
        })
        .collect())
}

fn parse_object(body: &[u8], name: Option<&str>) -> Result<StorageObject, SyncError> {
    let item: ItemPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("onedrive object is not json".into()))?;
    storage_object(item, name).ok_or_else(|| SyncError::Invalid("onedrive object name".into()))
}

fn status_to_error(status: u16) -> Result<(), SyncError> {
    match status {
        200 | 201 | 204 => Ok(()),
        404 => Err(SyncError::NotFound),
        409 | 412 => Err(SyncError::Precondition),
        _ => Err(SyncError::Invalid("onedrive request failed".into())),
    }
}

fn encode_object_path(name: &str) -> String {
    name.split('/')
        .map(|segment| {
            let mut out = String::new();
            for byte in segment.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(byte as char)
                    }
                    _ => out.push_str(&format!("%{byte:02X}")),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl<T: Transport> DriveClient for OneDriveStorage<T> {
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
            .send("GET", &Self::content_url(&found.name), &[], None)?;
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
            &Self::item_url(&found.name),
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
#[path = "onedrive_tests.rs"]
mod tests;
