use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FileLocatorError {
    #[error("Invalid test identifier URL: {0}")]
    InvalidTestIdentifierUrl(String),

    #[error("File not found for class: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
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

    /// Locate the Swift file for a given test identifier URL by searching for the class name
    ///
    /// Examples:
    /// - test_identifier_url: "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample"
    ///   workspace_path: "../AutoFixSampler"
    ///   Searches for: "AutoFixSamplerUITests.swift"
    ///
    /// - test_identifier_url: "test://com.apple.xcode/MyApp/MyUITests/Features/Login/Screens/LoginScreenTests/testLoginFlow"
    ///   workspace_path: "../MyApp"
    ///   Searches for: "LoginScreenTests.swift"
    pub fn locate_file(&self, test_identifier_url: &str) -> Result<PathBuf, FileLocatorError> {
        // Extract the class name from the test identifier URL
        let class_name = self.extract_class_name(test_identifier_url)?;

        // Search for the file in the workspace
        let file_name = format!("{}.swift", class_name);

        match self.search_for_file(&self.workspace_path, &file_name)? {
            Some(path) => Ok(path),
            None => Err(FileLocatorError::FileNotFound(class_name)),
        }
    }

    /// Extract the class name from a test identifier URL
    /// The class name is the second-to-last component (before the test method name)
    ///
    /// Example: "test://com.apple.xcode/MyApp/MyUITests/Features/Login/Screens/LoginScreenTests/testLoginFlow"
    /// Returns: "LoginScreenTests"
    fn extract_class_name(&self, test_identifier_url: &str) -> Result<String, FileLocatorError> {
        let parts = self.parse_test_identifier_url(test_identifier_url)?;

        // We need at least: scheme, project, target, class, testMethod = 5 parts
        if parts.len() < 5 {
            return Err(FileLocatorError::InvalidTestIdentifierUrl(
                test_identifier_url.to_string(),
            ));
        }

        // The class name is the second-to-last component (before the test method)
        let class_name = &parts[parts.len() - 2];

        Ok(class_name.to_string())
    }

    /// Recursively search for a file with the given name in the directory
    /// Uses case-sensitive matching
    fn search_for_file(
        &self,
        dir: &Path,
        file_name: &str,
    ) -> Result<Option<PathBuf>, FileLocatorError> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(None);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name()
                    && name == file_name {
                        return Ok(Some(path));
                    }
            } else if path.is_dir() {
                // Recursively search subdirectories
                if let Some(found) = self.search_for_file(&path, file_name)? {
                    return Ok(Some(found));
                }
            }
        }

        Ok(None)
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

    #[test]
    fn test_extract_class_name() {
        let locator = XCWorkspaceFileLocator::new("/tmp/workspace");

        // Simple case
        let url = "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample";
        assert_eq!(
            locator.extract_class_name(url).unwrap(),
            "AutoFixSamplerUITests"
        );

        // Nested case
        let url = "test://com.apple.xcode/MyApp/MyUITests/Features/Login/Screens/LoginScreenTests/testLoginFlow";
        assert_eq!(locator.extract_class_name(url).unwrap(), "LoginScreenTests");
    }

    #[test]
    fn test_locate_file_in_nested_directory() {
        // Create a nested workspace structure
        let temp_dir = std::env::temp_dir().join("test_workspace_search");
        let nested_path = temp_dir
            .join("MyUITests")
            .join("Features")
            .join("Login")
            .join("Screens");
        fs::create_dir_all(&nested_path).unwrap();

        let test_file = nested_path.join("LoginScreenTests.swift");
        fs::write(&test_file, "class LoginScreenTests { }").unwrap();

        let locator = XCWorkspaceFileLocator::new(&temp_dir);
        let url = "test://com.apple.xcode/MyApp/MyUITests/Features/Login/Screens/LoginScreenTests/testLoginFlow";

        let result = locator.locate_file(url).unwrap();
        assert_eq!(result, test_file);

        // Clean up
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_search_finds_file_anywhere_in_workspace() {
        // Create a workspace with file in deep subdirectory
        let temp_dir = std::env::temp_dir().join("test_workspace_deep_search");
        let deep_path = temp_dir.join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep_path).unwrap();

        let test_file = deep_path.join("MyTestClass.swift");
        fs::write(&test_file, "class MyTestClass { }").unwrap();

        let locator = XCWorkspaceFileLocator::new(&temp_dir);
        let url = "test://com.apple.xcode/Project/Target/MyTestClass/testSomething";

        let result = locator.locate_file(url).unwrap();
        assert_eq!(result, test_file);

        // Clean up
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
