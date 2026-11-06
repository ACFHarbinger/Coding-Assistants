//! Remote layout helpers: hashed device folders plus replica/manifests prefixes.

use super::types::{DeviceId, RemotePrefix};
use super::SyncError;

pub fn device_prefix(device: &DeviceId) -> Result<RemotePrefix, SyncError> {
    RemotePrefix::new(format!("devices/{}/", device.remote_folder_name()))
}

pub fn replica_prefix() -> RemotePrefix {
    RemotePrefix::new("replica/").expect("static replica prefix")
}

pub fn manifests_prefix() -> RemotePrefix {
    RemotePrefix::new("manifests/").expect("static manifests prefix")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::types::DeviceId;

    #[test]
    fn device_prefix_uses_hashed_folder_not_uuid() {
        let id = DeviceId::parse("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap();
        let prefix = device_prefix(&id).unwrap();
        assert!(prefix.as_str().starts_with("devices/"));
        assert!(prefix.as_str().ends_with('/'));
        assert!(!prefix.as_str().contains("aaaa"));
        assert!(!prefix.as_str().contains("eeee"));
        assert_eq!(prefix.as_str().matches('/').count(), 2);
    }

    #[test]
    fn replica_and_manifest_prefixes_are_fixed() {
        assert_eq!(replica_prefix().as_str(), "replica/");
        assert_eq!(manifests_prefix().as_str(), "manifests/");
    }
}
