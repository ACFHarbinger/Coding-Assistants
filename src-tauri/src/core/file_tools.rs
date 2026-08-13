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

    pub async fn read_file(&self, relative_path: &str) -> Result<String, String> {
        let full_path = self.work_dir.join(relative_path);
        tokio::fs::read_to_string(full_path)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn write_file(&self, relative_path: &str, content: &str) -> Result<(), String> {
        let full_path = self.work_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::fs::write(full_path, content)
            .await
            .map_err(|e| e.to_string())
    }
}
