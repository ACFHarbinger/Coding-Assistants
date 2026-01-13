use std::fs;
use std::path::PathBuf;

pub struct FileTools {
    pub work_dir: PathBuf,
}

impl FileTools {
    pub fn new(work_dir: String) -> Self {
        Self {
            work_dir: PathBuf::from(work_dir),
        }
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
