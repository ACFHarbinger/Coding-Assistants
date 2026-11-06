//! Private Dropbox app-folder adapter (S12).
//!
//! Paths are approved prefixes plus hashed BlobIds under the app folder.
//! Never creates shared links or uploads the replica key.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::google_http::Transport;
use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use serde::Deserialize;
use serde_json::json;
use std::fmt;

const API: &str = "https://api.dropboxapi.com/2";
const CONTENT: &str = "https://content.dropboxapi.com/2";

pub struct DropboxStorage<T> {
    transport: T,
}

impl<T> fmt::Debug for DropboxStorage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DropboxStorage(***)")
    }
}

impl<T: Transport> DropboxStorage<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    fn object_path(prefix: &RemotePrefix, id: &BlobId) -> String {
        format!("/{}{}", prefix.as_str(), id.as_str())
    }

    fn lookup(&self, path: &str) -> Result<StorageObject, SyncError> {
        let body = json!({"path": path}).to_string();
        let response = self.transport.send(
            "POST",
            &format!("{API}/files/get_metadata"),
            &[("Content-Type", "application/json")],
            Some(body.as_bytes()),
        )?;
        if response.status == 409 || response.status == 404 {
            return Err(SyncError::NotFound);
        }
        status_to_error(response.status)?;
        parse_entry(&response.body)
    }

    fn list_objects(&self, prefix: &str) -> Result<Vec<StorageObject>, SyncError> {
        let path = match prefix.trim_end_matches('/') {
            "" => String::new(),
            folder => format!("/{folder}"),
        };
        let body = json!({"path": path, "recursive": prefix.is_empty()}).to_string();
        let response = self.transport.send(
            "POST",
            &format!("{API}/files/list_folder"),
            &[("Content-Type", "application/json")],
            Some(body.as_bytes()),
        )?;
        if response.status == 409 || response.status == 404 {
            return Ok(Vec::new());
        }
        status_to_error(response.status)?;
        parse_entry_list(&response.body)
    }

    fn upload(&self, path: &str, bytes: &[u8], expected: Option<&ETag>) -> Result<ETag, SyncError> {
        let mode = match expected {
            Some(etag) => json!({".tag": "update", "update": etag.as_str()}),
            None => json!("add"),
        };
        let arg = json!({"path": path, "mode": mode, "autorename": false}).to_string();
        let response = self.transport.send(
            "POST",
            &format!("{CONTENT}/files/upload"),
            &[
                ("Content-Type", "application/octet-stream"),
                ("Dropbox-API-Arg", arg.as_str()),
            ],
            Some(bytes),
        )?;
        status_to_error(response.status)?;
        parse_entry(&response.body).map(|item| item.object.etag)
    }

    fn find(&self, id: &BlobId) -> Result<StorageObject, SyncError> {
        self.list_objects("")?
            .into_iter()
            .find(|item| item.object.blob_id == *id)
            .ok_or(SyncError::NotFound)
    }
}

struct StorageObject {
    path: String,
    object: RemoteObject,
}

#[derive(Deserialize)]
struct ListPayload {
    #[serde(default)]
    entries: Vec<EntryPayload>,
}

#[derive(Deserialize)]
struct EntryPayload {
    #[serde(default, rename = ".tag")]
    tag: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path_display: String,
    #[serde(default)]
    rev: String,
    #[serde(default)]
    size: serde_json::Value,
}

fn json_size(value: &serde_json::Value) -> u64 {
    match value {
        serde_json::Value::String(text) => text.parse().unwrap_or(0),
        serde_json::Value::Number(number) => number.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn storage_object(entry: EntryPayload) -> Option<StorageObject> {
    if entry.tag == "folder" {
        return None;
    }
    let path = if entry.path_display.starts_with('/') {
        entry.path_display.clone()
    } else if entry.path_display.is_empty() {
        format!("/{}", entry.name)
    } else {
        format!("/{}", entry.path_display)
    };
    let blob_id = BlobId::parse(path.rsplit('/').next()?).ok()?;
    if entry.rev.is_empty() {
        return None;
    }
    Some(StorageObject {
        path,
        object: RemoteObject {
            blob_id,
            etag: ETag::new(entry.rev),
            size: json_size(&entry.size),
        },
    })
}

fn parse_entry_list(body: &[u8]) -> Result<Vec<StorageObject>, SyncError> {
    let parsed: ListPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("dropbox list is not json".into()))?;
    Ok(parsed
        .entries
        .into_iter()
        .filter_map(storage_object)
        .collect())
}

fn parse_entry(body: &[u8]) -> Result<StorageObject, SyncError> {
    let entry: EntryPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("dropbox object is not json".into()))?;
    storage_object(entry).ok_or_else(|| SyncError::Invalid("dropbox object name".into()))
}

fn status_to_error(status: u16) -> Result<(), SyncError> {
    match status {
        200 | 201 | 204 => Ok(()),
        404 => Err(SyncError::NotFound),
        409 | 412 => Err(SyncError::Precondition),
        _ => Err(SyncError::Invalid("dropbox request failed".into())),
    }
}

impl<T: Transport> DriveClient for DropboxStorage<T> {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        Ok(self
            .list_objects(prefix.as_str())?
            .into_iter()
            .map(|item| item.object)
            .collect())
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        let found = self.find(id)?;
        let arg = json!({"path": found.path}).to_string();
        let response = self.transport.send(
            "POST",
            &format!("{CONTENT}/files/download"),
            &[("Dropbox-API-Arg", arg.as_str())],
            None,
        )?;
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
        let path = Self::object_path(prefix, id);
        let existing = match self.lookup(&path) {
            Ok(item) => Some(item),
            Err(SyncError::NotFound) => None,
            Err(error) => return Err(error),
        };
        match (existing, expected) {
            (None, None) => self.upload(&path, bytes, None),
            (Some(item), Some(etag)) if item.object.etag == *etag => {
                self.upload(&path, bytes, Some(etag))
            }
            _ => Err(SyncError::Precondition),
        }
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let found = self.find(id)?;
        if found.object.etag != *etag {
            return Err(SyncError::Precondition);
        }
        let body = json!({"path": found.path, "parent_rev": etag.as_str()}).to_string();
        let response = self.transport.send(
            "POST",
            &format!("{API}/files/delete_v2"),
            &[("Content-Type", "application/json")],
            Some(body.as_bytes()),
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
#[path = "dropbox_tests.rs"]
mod tests;
