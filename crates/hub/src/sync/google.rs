//! Google Drive `drive.appdata` adapter (S3 / #93).
//!
//! Talks Drive v3 through a [`Transport`] so unit tests never hit the network.
//! File names are BlobIds only. Credentials are not in `Debug` or error text.

use super::client::{DriveClient, RemoteObject, ReplicaAdvance};
use super::google_http::Transport;
use super::types::{BlobId, ETag, RemotePrefix};
use super::SyncError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";

pub struct GoogleDrive<T> {
    transport: T,
    folders: HashMap<String, String>,
}

impl<T> fmt::Debug for GoogleDrive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GoogleDrive(***)")
    }
}

impl<T: Transport> GoogleDrive<T> {
    pub fn new(transport: T, folders: HashMap<RemotePrefix, String>) -> Self {
        let folders = folders
            .into_iter()
            .map(|(prefix, id)| (prefix.as_str().to_string(), id))
            .collect();
        Self { transport, folders }
    }

    fn folder_id(&self, prefix: &RemotePrefix) -> Result<&str, SyncError> {
        self.folders
            .get(prefix.as_str())
            .map(String::as_str)
            .ok_or_else(|| SyncError::Invalid("unknown drive folder prefix".into()))
    }

    fn lookup(&self, prefix: &RemotePrefix, id: &BlobId) -> Result<DriveFile, SyncError> {
        let folder = self.folder_id(prefix)?;
        let url = format!(
            "{FILES_URL}?spaces=appDataFolder&fields=files(id,name,size,etag)&q={}",
            list_query(folder, Some(id))
        );
        let response = self.transport.send("GET", &url, &[], None)?;
        status_to_error(response.status)?;
        let files = parse_file_list(&response.body)?;
        files.into_iter().next().ok_or(SyncError::NotFound)
    }
}

#[derive(Clone, Debug)]
struct DriveFile {
    google_id: String,
    object: RemoteObject,
}

#[derive(Deserialize)]
struct FilesPayload {
    #[serde(default)]
    files: Vec<FilePayload>,
}

#[derive(Deserialize)]
struct FilePayload {
    id: String,
    name: String,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    etag: Option<String>,
}

fn list_query(folder_id: &str, name: Option<&BlobId>) -> String {
    let mut query = format!("'{folder_id}' in parents and trashed = false");
    if let Some(blob) = name {
        query.push_str(&format!(" and name = '{}'", blob.as_str()));
    }
    url_encode_query(&query)
}

fn parse_file_list(body: &[u8]) -> Result<Vec<DriveFile>, SyncError> {
    let parsed: FilesPayload = serde_json::from_slice(body)
        .map_err(|_| SyncError::Invalid("google drive list is not json".into()))?;
    let mut out = Vec::new();
    for file in parsed.files {
        let Ok(blob_id) = BlobId::parse(&file.name) else {
            continue;
        };
        let size = file
            .size
            .as_deref()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        out.push(DriveFile {
            google_id: file.id,
            object: RemoteObject {
                blob_id,
                etag: ETag::new(file.etag.unwrap_or_default()),
                size,
            },
        });
    }
    Ok(out)
}

fn status_to_error(status: u16) -> Result<(), SyncError> {
    match status {
        200 | 201 | 204 => Ok(()),
        404 => Err(SyncError::NotFound),
        412 => Err(SyncError::Precondition),
        _ => Err(SyncError::Invalid("google drive request failed".into())),
    }
}

fn url_encode_query(query: &str) -> String {
    let mut out = String::new();
    for byte in query.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

impl<T: Transport> DriveClient for GoogleDrive<T> {
    fn list(&self, prefix: &RemotePrefix) -> Result<Vec<RemoteObject>, SyncError> {
        let folder = self.folder_id(prefix)?;
        let url = format!(
            "{FILES_URL}?spaces=appDataFolder&fields=files(id,name,size,etag)&q={}",
            list_query(folder, None)
        );
        let response = self.transport.send("GET", &url, &[], None)?;
        status_to_error(response.status)?;
        Ok(parse_file_list(&response.body)?
            .into_iter()
            .map(|file| file.object)
            .collect())
    }

    fn get(&self, id: &BlobId) -> Result<Vec<u8>, SyncError> {
        let url = format!(
            "{FILES_URL}?spaces=appDataFolder&fields=files(id,name,size,etag)&q={}",
            url_encode_query(&format!("name = '{}' and trashed = false", id.as_str()))
        );
        let response = self.transport.send("GET", &url, &[], None)?;
        status_to_error(response.status)?;
        let file = parse_file_list(&response.body)?
            .into_iter()
            .next()
            .ok_or(SyncError::NotFound)?;
        let media = format!("{FILES_URL}/{}?alt=media", file.google_id);
        let response = self.transport.send("GET", &media, &[], None)?;
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
        let existing = match self.lookup(prefix, id) {
            Ok(file) => Some(file),
            Err(SyncError::NotFound) => None,
            Err(error) => return Err(error),
        };
        match (existing, expected) {
            (None, None) => self.create(prefix, id, bytes),
            (Some(file), Some(etag)) if file.object.etag == *etag => {
                self.replace(&file.google_id, etag, bytes)
            }
            _ => Err(SyncError::Precondition),
        }
    }

    fn delete_if_match(&mut self, id: &BlobId, etag: &ETag) -> Result<(), SyncError> {
        let url = format!(
            "{FILES_URL}?spaces=appDataFolder&fields=files(id,name,size,etag)&q={}",
            url_encode_query(&format!("name = '{}' and trashed = false", id.as_str()))
        );
        let response = self.transport.send("GET", &url, &[], None)?;
        status_to_error(response.status)?;
        let file = parse_file_list(&response.body)?
            .into_iter()
            .next()
            .ok_or(SyncError::NotFound)?;
        if file.object.etag != *etag {
            return Err(SyncError::Precondition);
        }
        let url = format!("{FILES_URL}/{}", file.google_id);
        let if_match = file.object.etag.as_str().to_string();
        let response =
            self.transport
                .send("DELETE", &url, &[("If-Match", if_match.as_str())], None)?;
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
            let listed = self.list(&prefix)?;
            for object in listed {
                self.delete_if_match(&object.blob_id, &object.etag)?;
            }
        }
        Ok(())
    }
}

impl<T: Transport> GoogleDrive<T> {
    fn create(&self, prefix: &RemotePrefix, id: &BlobId, bytes: &[u8]) -> Result<ETag, SyncError> {
        let folder = self.folder_id(prefix)?;
        let metadata = serde_json::json!({
            "name": id.as_str(),
            "parents": [folder],
        })
        .to_string();
        let url = format!("{UPLOAD_URL}?uploadType=multipart&spaces=appDataFolder");
        let response = self.transport.send(
            "POST",
            &url,
            &[("Content-Type", "application/json")],
            Some(metadata.as_bytes()),
        )?;
        status_to_error(response.status)?;
        let created: FilePayload = serde_json::from_slice(&response.body)
            .map_err(|_| SyncError::Invalid("google drive create is not json".into()))?;
        self.replace(&created.id, &ETag::new(String::new()), bytes)
    }

    fn replace(&self, google_id: &str, etag: &ETag, bytes: &[u8]) -> Result<ETag, SyncError> {
        let url = format!("{UPLOAD_URL}/{google_id}?uploadType=media");
        let headers = if etag.as_str().is_empty() {
            vec![("Content-Type", "application/octet-stream")]
        } else {
            vec![
                ("Content-Type", "application/octet-stream"),
                ("If-Match", etag.as_str()),
            ]
        };
        let response = self.transport.send("PATCH", &url, &headers, Some(bytes))?;
        status_to_error(response.status)?;
        Ok(ETag::new(response.etag.unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::google_http::{HttpResponse, ScriptedTransport};
    use super::*;
    use crate::sync::layout::replica_prefix;

    fn hex_id(seed: u8) -> BlobId {
        BlobId::parse(&format!("{seed:064x}")).unwrap()
    }

    #[test]
    fn parse_file_list_keeps_hashed_names_only() {
        let body = br#"{
            "files": [
                {"id": "g1", "name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size": "4", "etag": "e1"},
                {"id": "g2", "name": "journals/claude.md", "size": "9", "etag": "e2"}
            ]
        }"#;
        let files = parse_file_list(body).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].object.blob_id.as_str().len(), 64);
        assert!(!files[0].object.blob_id.as_str().contains('/'));
    }

    #[test]
    fn debug_and_errors_omit_tokens() {
        let drive = GoogleDrive::new(ScriptedTransport::new(Vec::new()), HashMap::new());
        assert_eq!(format!("{drive:?}"), "GoogleDrive(***)");
        let err = SyncError::Invalid("google drive request failed".into());
        assert!(!format!("{err}").contains("Bearer"));
        assert!(!format!("{err}").contains("ya29"));
    }

    #[test]
    fn list_and_put_use_appdata_and_hashed_names() {
        let blob = hex_id(0xaa);
        let folder = replica_prefix();
        let mut folders = HashMap::new();
        folders.insert(folder.clone(), "folder1".into());
        let list_body = format!(
            r#"{{"files":[{{"id":"g1","name":"{}","size":"3","etag":"e1"}}]}}"#,
            blob.as_str()
        );
        let transport = ScriptedTransport::new(vec![
            HttpResponse {
                status: 200,
                etag: None,
                body: list_body.into_bytes(),
            },
            HttpResponse {
                status: 200,
                etag: None,
                body: br#"{"files":[]}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                etag: None,
                body: br#"{"id":"g-new","name":"x"}"#.to_vec(),
            },
            HttpResponse {
                status: 200,
                etag: Some("e-new".into()),
                body: Vec::new(),
            },
        ]);
        let mut drive = GoogleDrive::new(transport, folders);
        let listed = drive.list(&folder).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].blob_id, blob);

        let created = hex_id(0xbb);
        drive
            .put_if_unmodified(&folder, &created, b"enc", None)
            .unwrap();
        let recorded = drive.transport.recorded.lock().unwrap().clone();
        assert!(recorded.iter().any(|line| line.contains("appDataFolder")));
        assert!(recorded
            .iter()
            .any(|line| line.contains(created.as_str()) && !line.contains("enc")));
        assert!(recorded.iter().all(|line| !line.contains("Bearer")));
        assert!(recorded.iter().all(|line| !line.contains("ya29")));
    }

    #[test]
    fn stale_etag_does_not_replace() {
        let blob = hex_id(0xcc);
        let folder = replica_prefix();
        let mut folders = HashMap::new();
        folders.insert(folder.clone(), "folder1".into());
        let list_body = format!(
            r#"{{"files":[{{"id":"g1","name":"{}","size":"3","etag":"old"}}]}}"#,
            blob.as_str()
        );
        let transport = ScriptedTransport::new(vec![HttpResponse {
            status: 200,
            etag: None,
            body: list_body.into_bytes(),
        }]);
        let mut drive = GoogleDrive::new(transport, folders);
        let err = drive
            .put_if_unmodified(&folder, &blob, b"x", Some(&ETag::new("nope")))
            .unwrap_err();
        assert_eq!(err, SyncError::Precondition);
    }

    #[test]
    fn drive_http_412_is_precondition() {
        let blob = hex_id(0xdd);
        let folder = replica_prefix();
        let mut folders = HashMap::new();
        folders.insert(folder.clone(), "folder1".into());
        let transport = ScriptedTransport::new(vec![HttpResponse {
            status: 412,
            etag: None,
            body: Vec::new(),
        }]);
        let mut drive = GoogleDrive::new(transport, folders);
        let err = drive
            .put_if_unmodified(&folder, &blob, b"x", None)
            .unwrap_err();
        assert_eq!(err, SyncError::Precondition);
        assert!(!format!("{err}").contains("Bearer"));
    }
}
