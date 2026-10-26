//! Open without following links, then keep that handle through sampling/hashing.
use crate::policy::{classify, Config, RootKind};
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path};

pub(super) struct CheckedFile {
    pub file: File,
    // Windows directory handles deny deletion/rename until hashing finishes.
    _parents: Vec<File>,
}

impl CheckedFile {
    pub fn open(
        config: &Config,
        raw: &str,
        write: bool,
    ) -> Result<Option<(RootKind, std::path::PathBuf, Self)>, String> {
        let (kind, path) = classify(config, raw, write)?;
        let opened = match open_no_links(&path) {
            Ok(opened) => opened,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("validated file open: {error}")),
        };
        opened.revalidate(config, raw, write)?;
        Ok(Some((kind, path, opened)))
    }

    pub fn revalidate(&self, config: &Config, raw: &str, write: bool) -> Result<(), String> {
        let (_, path) = classify(config, raw, write)?;
        let meta = self.file.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err("path is not a regular file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let current = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            if meta.nlink() != 1 || current.dev() != meta.dev() || current.ino() != meta.ino() {
                return Err("file identity changed or hard links are present".into());
            }
        }
        #[cfg(windows)]
        {
            let _ = path;
            if crate::policy::windows_handle_links(&self.file)? != 1 {
                return Err("hard links are rejected".into());
            }
        }
        #[cfg(not(any(unix, windows)))]
        return Err("validated file handles are unsupported on this platform".into());
        Ok(())
    }
}

#[cfg(unix)]
fn open_no_links(path: &Path) -> io::Result<CheckedFile> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    let components: Vec<_> = path.components().collect();
    let mut directory = File::open("/")?;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            continue;
        };
        let name = CString::new(name.as_bytes()).map_err(|_| io::Error::other("invalid path"))?;
        let last = index + 1 == components.len();
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if last { 0 } else { libc::O_DIRECTORY };
        // SAFETY: directory is live, name is NUL-terminated; no create flag is used.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openat returned a new owned descriptor.
        let file = unsafe { File::from_raw_fd(fd) };
        if last {
            return Ok(CheckedFile {
                file,
                _parents: vec![],
            });
        }
        directory = file;
    }
    Err(io::Error::other("path must name a file"))
}

#[cfg(windows)]
fn open_no_links(path: &Path) -> io::Result<CheckedFile> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    const REPARSE: u32 = 0x400;
    const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let mut parents = Vec::new();
    let mut current = std::path::PathBuf::new();
    let components: Vec<_> = path.components().collect();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        if !matches!(component, Component::Normal(_)) {
            continue;
        }
        // Deny FILE_SHARE_DELETE: components cannot be replaced while held.
        let file = fs::OpenOptions::new()
            .read(true)
            .share_mode(0x1 | 0x2)
            .custom_flags(OPEN_REPARSE_POINT | BACKUP_SEMANTICS)
            .open(&current)?;
        if file.metadata()?.file_attributes() & REPARSE != 0 {
            return Err(io::Error::other("reparse points are rejected"));
        }
        if index + 1 == components.len() {
            return Ok(CheckedFile {
                file,
                _parents: parents,
            });
        }
        parents.push(file);
    }
    Err(io::Error::other("path must name a file"))
}

#[cfg(not(any(unix, windows)))]
fn open_no_links(_: &Path) -> io::Result<CheckedFile> {
    Err(io::Error::other(
        "validated file handles are unsupported on this platform",
    ))
}
