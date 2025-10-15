use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeEditorTool {
    name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeEditorInput {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeEditorResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

impl CodeEditorTool {
    pub fn new() -> Self {
        Self {
            name: "code_editor".to_string(),
            description: r#"A tool to edit source code files within the workspace.
This tool performs exact string replacement in files.

Input format:
{
  "file_path": "relative/path/to/file.swift",
  "old_content": "exact string to replace",
  "new_content": "new string content"
}

The tool will:
1. Read the file
2. Verify the old_content exists exactly as specified
3. Replace it with new_content
4. Write the file back

IMPORTANT: The old_content must match exactly (including whitespace and indentation)."#
                .to_string(),
        }
    }

    pub fn to_anthropic_tool(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to the file within the workspace"
                    },
                    "old_content": {
                        "type": "string",
                        "description": "Exact content to be replaced"
                    },
                    "new_content": {
                        "type": "string",
                        "description": "New content to replace with"
                    }
                },
                "required": ["file_path", "old_content", "new_content"]
            }
        })
    }

    pub fn execute(&self, input: CodeEditorInput, workspace_root: &Path) -> CodeEditorResult {
        let full_path = workspace_root.join(&input.file_path);

        // Read the current file content
        let current_content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => {
                return CodeEditorResult {
                    success: false,
                    message: format!("Failed to read file: {}", full_path.display()),
                    error: Some(e.to_string()),
                };
            }
        };

        // Check if old_content exists in the file
        if !current_content.contains(&input.old_content) {
            return CodeEditorResult {
                success: false,
                message: format!(
                    "Old content not found in file: {}",
                    full_path.display()
                ),
                error: Some("The exact old_content string was not found in the file. Make sure it matches exactly including whitespace.".to_string()),
            };
        }

        // Perform the replacement
        let new_content = current_content.replace(&input.old_content, &input.new_content);

        // Write the new content back
        match fs::write(&full_path, new_content) {
            Ok(_) => CodeEditorResult {
                success: true,
                message: format!("Successfully edited file: {}", full_path.display()),
                error: None,
            },
            Err(e) => CodeEditorResult {
                success: false,
                message: format!("Failed to write file: {}", full_path.display()),
                error: Some(e.to_string()),
            },
        }
    }
}

impl Default for CodeEditorTool {
    fn default() -> Self {
        Self::new()
    }
}
