use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FileLocatorError {
    #[error("Invalid test identifier URL: {0}")]
    InvalidTestIdentifierUrl(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
}

pub struct XCWorkspaceFileLocator {
    workspace_path: PathBuf,
}

impl XCWorkspaceFileLocator {
    pub fn new<P: AsRef<Path>>(workspace_path: P) -> Self {
        Self {
            workspace_path: workspace_path.as_ref().to_path_buf(),
        }
    }

    /// Locate the Swift file for a given test identifier URL
    ///
    /// Example:
    /// test_identifier_url: "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample"
    /// workspace_path: "../AutoFixSampler"
    /// Returns: "../AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests.swift"
    pub fn locate_file(&self, test_identifier_url: &str) -> Result<PathBuf, FileLocatorError> {
        // Parse the test identifier URL
        let parts = self.parse_test_identifier_url(test_identifier_url)?;

        // Extract the target and file information
        // Format: test://com.apple.xcode/ProjectName/TargetName/FileName/testMethodName
        if parts.len() < 4 {
            return Err(FileLocatorError::InvalidTestIdentifierUrl(
                test_identifier_url.to_string(),
            ));
        }

        let target_name = &parts[2]; // e.g., "AutoFixSamplerUITests"
        let file_name = &parts[3]; // e.g., "AutoFixSamplerUITests"

        // Construct the file path: workspace/TargetName/FileName.swift
        let file_path = self
            .workspace_path
            .join(target_name)
            .join(format!("{}.swift", file_name));

        // Verify the file exists
        if !file_path.exists() {
            return Err(FileLocatorError::FileNotFound(file_path));
        }

        Ok(file_path)
    }

    /// Parse the test identifier URL into parts
    fn parse_test_identifier_url(&self, url: &str) -> Result<Vec<String>, FileLocatorError> {
        // Remove the "test://" prefix
        let without_prefix = url
            .strip_prefix("test://")
            .ok_or_else(|| FileLocatorError::InvalidTestIdentifierUrl(url.to_string()))?;

        // Split by '/' and collect parts
        let parts: Vec<String> = without_prefix.split('/').map(|s| s.to_string()).collect();

        if parts.is_empty() {
            return Err(FileLocatorError::InvalidTestIdentifierUrl(url.to_string()));
        }

        Ok(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_test_identifier_url() {
        let locator = XCWorkspaceFileLocator::new("/tmp/workspace");
        let url = "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample";

        let parts = locator.parse_test_identifier_url(url).unwrap();

        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0], "com.apple.xcode");
        assert_eq!(parts[1], "AutoFixSampler");
        assert_eq!(parts[2], "AutoFixSamplerUITests");
        assert_eq!(parts[3], "AutoFixSamplerUITests");
        assert_eq!(parts[4], "testExample");
    }

    #[test]
    fn test_parse_invalid_url() {
        let locator = XCWorkspaceFileLocator::new("/tmp/workspace");
        let url = "invalid://url";

        let result = locator.parse_test_identifier_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_locate_file() {
        // Create a temporary workspace structure
        let temp_dir = std::env::temp_dir().join("test_workspace");
        let target_dir = temp_dir.join("AutoFixSamplerUITests");
        fs::create_dir_all(&target_dir).unwrap();

        let test_file = target_dir.join("AutoFixSamplerUITests.swift");
        fs::write(&test_file, "// Test file").unwrap();

        let locator = XCWorkspaceFileLocator::new(&temp_dir);
        let url = "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample";

        let result = locator.locate_file(url).unwrap();
        assert_eq!(result, test_file);

        // Clean up
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_locate_nonexistent_file() {
        let locator = XCWorkspaceFileLocator::new("/nonexistent/workspace");
        let url = "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample";

        let result = locator.locate_file(url);
        assert!(result.is_err());
        match result {
            Err(FileLocatorError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }
    }
}
