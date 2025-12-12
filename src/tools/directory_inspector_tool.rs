use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInspectorTool {
    name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInspectorInput {
    pub operation: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInspectorResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl DirectoryInspectorTool {
    pub fn new() -> Self {
        Self {
            name: "directory_inspector".to_string(),
            description: r#"A tool to inspect the file system, read files, and search for content.
Operations:
- "list": List files and directories in a path. Returns array of {name, type, path}.
- "read": Read the contents of a file. Returns {content: string}.
- "search": Search for a pattern (regex) in files. Returns array of {file, line, content, line_number}.
- "find": Find files by name pattern (glob). Returns array of file paths.

Input format: {"operation": "list|read|search|find", "path": "/path/to/dir", "pattern": "optional search pattern"}"#.to_string(),
        }
    }

    pub fn to_tool_definition(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["list", "read", "search", "find"],
                        "description": "The operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "The file or directory path"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Optional search pattern (regex for search, glob for find)"
                    }
                },
                "required": ["operation", "path"]
            }
        })
    }

    pub fn execute(
        &self,
        input: DirectoryInspectorInput,
        workspace_root: &Path,
    ) -> DirectoryInspectorResult {
        let full_path = workspace_root.join(&input.path);

        match input.operation.as_str() {
            "list" => self.list_directory(&full_path),
            "read" => self.read_file(&full_path),
            "search" => {
                if let Some(pattern) = input.pattern {
                    self.search_files(&full_path, &pattern)
                } else {
                    DirectoryInspectorResult {
                        success: false,
                        data: None,
                        error: Some("Pattern is required for search operation".to_string()),
                    }
                }
            }
            "find" => {
                if let Some(pattern) = input.pattern {
                    self.find_files(&full_path, &pattern)
                } else {
                    DirectoryInspectorResult {
                        success: false,
                        data: None,
                        error: Some("Pattern is required for find operation".to_string()),
                    }
                }
            }
            _ => DirectoryInspectorResult {
                success: false,
                data: None,
                error: Some(format!("Unknown operation: {}", input.operation)),
            },
        }
    }

    fn list_directory(&self, path: &Path) -> DirectoryInspectorResult {
        match fs::read_dir(path) {
            Ok(entries) => {
                let items: Vec<serde_json::Value> = entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| {
                        let path = entry.path();
                        let file_type = if path.is_dir() { "directory" } else { "file" };
                        serde_json::json!({
                            "name": entry.file_name().to_string_lossy(),
                            "type": file_type,
                            "path": path.to_string_lossy()
                        })
                    })
                    .collect();

                DirectoryInspectorResult {
                    success: true,
                    data: Some(serde_json::json!(items)),
                    error: None,
                }
            }
            Err(e) => DirectoryInspectorResult {
                success: false,
                data: None,
                error: Some(format!("Failed to list directory: {}", e)),
            },
        }
    }

    fn read_file(&self, path: &Path) -> DirectoryInspectorResult {
        match fs::read_to_string(path) {
            Ok(content) => DirectoryInspectorResult {
                success: true,
                data: Some(serde_json::json!({"content": content})),
                error: None,
            },
            Err(e) => DirectoryInspectorResult {
                success: false,
                data: None,
                error: Some(format!("Failed to read file: {}", e)),
            },
        }
    }

    fn search_files(&self, path: &Path, pattern: &str) -> DirectoryInspectorResult {
        let regex = match regex::Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => {
                return DirectoryInspectorResult {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid regex pattern: {}", e)),
                };
            }
        };

        let mut results = Vec::new();
        if let Err(e) = self.search_in_directory(path, &regex, &mut results) {
            return DirectoryInspectorResult {
                success: false,
                data: None,
                error: Some(format!("Search failed: {}", e)),
            };
        }

        DirectoryInspectorResult {
            success: true,
            data: Some(serde_json::json!(results)),
            error: None,
        }
    }

    fn search_in_directory(
        &self,
        path: &Path,
        regex: &regex::Regex,
        results: &mut Vec<serde_json::Value>,
    ) -> std::io::Result<()> {
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        results.push(serde_json::json!({
                            "file": path.to_string_lossy(),
                            "line_number": line_num + 1,
                            "content": line
                        }));
                    }
                }
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                // Skip hidden files and common build directories
                if let Some(name) = entry_path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.') || name_str == "build" || name_str == "DerivedData"
                    {
                        continue;
                    }
                }
                self.search_in_directory(&entry_path, regex, results)?;
            }
        }
        Ok(())
    }

    fn find_files(&self, path: &Path, pattern: &str) -> DirectoryInspectorResult {
        let glob_pattern = if path.is_dir() {
            format!("{}/**/{}", path.to_string_lossy(), pattern)
        } else {
            pattern.to_string()
        };

        match glob::glob(&glob_pattern) {
            Ok(paths) => {
                let files: Vec<String> = paths
                    .filter_map(|entry| entry.ok())
                    .map(|path| path.to_string_lossy().to_string())
                    .collect();

                DirectoryInspectorResult {
                    success: true,
                    data: Some(serde_json::json!(files)),
                    error: None,
                }
            }
            Err(e) => DirectoryInspectorResult {
                success: false,
                data: None,
                error: Some(format!("Glob pattern error: {}", e)),
            },
        }
    }
}

impl Default for DirectoryInspectorTool {
    fn default() -> Self {
        Self::new()
    }
}
