use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum AttachmentHandlerError {
    #[error("Failed to execute xcresulttool: {0}")]
    ExecutionError(String),

    #[error("xcresulttool returned non-zero exit code: {0}")]
    NonZeroExitCode(i32),

    #[error("Failed to create output directory: {0}")]
    CreateDirectoryError(#[from] std::io::Error),

    #[error("No attachments found")]
    NoAttachmentsFound,
}

pub struct XCTestResultAttachmentHandler {
    xcresulttool_path: PathBuf,
}

impl XCTestResultAttachmentHandler {
    pub fn new() -> Self {
        Self {
            xcresulttool_path: PathBuf::from("xcrun"),
        }
    }

    /// Fetch attachments for a test and keep only the newest one
    pub fn fetch_attachments<P: AsRef<Path>>(
        &self,
        test_id: &str,
        xcresult_path: P,
        output_path: P,
    ) -> Result<PathBuf, AttachmentHandlerError> {
        let output_dir = output_path.as_ref().join("attachments");

        // Create the attachments directory
        fs::create_dir_all(&output_dir)?;

        // Execute xcresulttool to export attachments
        let output = Command::new(&self.xcresulttool_path)
            .arg("xcresulttool")
            .arg("export")
            .arg("attachments")
            .arg("--test-id")
            .arg(test_id)
            .arg("--path")
            .arg(xcresult_path.as_ref())
            .arg("--output-path")
            .arg(&output_dir)
            .output()
            .map_err(|e| AttachmentHandlerError::ExecutionError(e.to_string()))?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(AttachmentHandlerError::NonZeroExitCode(exit_code));
        }

        // Find and keep only the newest attachment
        self.keep_newest_attachment(&output_dir)?;

        Ok(output_dir)
    }

    /// Keep only the newest attachment in the directory
    fn keep_newest_attachment(&self, dir: &Path) -> Result<(), AttachmentHandlerError> {
        let entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .collect();

        if entries.is_empty() {
            return Err(AttachmentHandlerError::NoAttachmentsFound);
        }

        // Find the newest file by modification time
        let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;

        for entry in &entries {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    match &newest {
                        None => newest = Some((entry.path(), modified)),
                        Some((_, newest_time)) if modified > *newest_time => {
                            newest = Some((entry.path(), modified));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Delete all files except the newest
        if let Some((newest_path, _)) = newest {
            for entry in entries {
                let path = entry.path();
                if path != newest_path {
                    fs::remove_file(&path)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for XCTestResultAttachmentHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_handler_creation() {
        let handler = XCTestResultAttachmentHandler::new();
        assert_eq!(handler.xcresulttool_path, PathBuf::from("xcrun"));
    }

    #[test]
    fn test_keep_newest_attachment() {
        use std::thread;
        use std::time::Duration;

        let temp_dir = std::env::temp_dir().join("test_attachments");
        fs::create_dir_all(&temp_dir).unwrap();

        // Create multiple files with different timestamps
        let file1 = temp_dir.join("old.png");
        let file2 = temp_dir.join("newer.png");
        let file3 = temp_dir.join("newest.png");

        File::create(&file1).unwrap().write_all(b"old").unwrap();
        thread::sleep(Duration::from_millis(10));
        File::create(&file2).unwrap().write_all(b"newer").unwrap();
        thread::sleep(Duration::from_millis(10));
        File::create(&file3).unwrap().write_all(b"newest").unwrap();

        let handler = XCTestResultAttachmentHandler::new();
        handler.keep_newest_attachment(&temp_dir).unwrap();

        // Only the newest file should remain
        assert!(!file1.exists());
        assert!(!file2.exists());
        assert!(file3.exists());

        // Clean up
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
