use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

pub struct FileTools {
    pub work_dir: PathBuf,
}

impl FileTools {
    pub fn new(work_dir: String) -> Self {
        Self {
            work_dir: PathBuf::from(work_dir),
        }
    }

    pub fn list_files(&self) -> Vec<FileInfo> {
        let mut files = Vec::new();
        for entry in WalkDir::new(&self.work_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                files.push(FileInfo {
                    path: entry
                        .path()
                        .strip_prefix(&self.work_dir)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string(),
                    is_dir: entry.file_type().is_dir(),
                    size: metadata.len(),
                });
            }
        }
        files
    }

    pub fn read_file(&self, relative_path: &str) -> Result<String, String> {
        let full_path = self.work_dir.join(relative_path);
        fs::read_to_string(full_path).map_err(|e| e.to_string())
    }

    pub fn write_file(&self, relative_path: &str, content: &str) -> Result<(), String> {
        let full_path = self.work_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(full_path, content).map_err(|e| e.to_string())
    }
}
