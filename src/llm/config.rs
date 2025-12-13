// Provider configuration types

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::env;

/// Supported LLM provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProviderType {
    #[default]
    Claude,
    OpenAI,
    Ollama,
}

impl ProviderType {
    /// Parse provider type from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "ollama" => Ok(ProviderType::Ollama),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}


/// Configuration for an LLM provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: SecretString,
    pub api_base: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub rate_limit_tpm: Option<u32>,
}

impl ProviderConfig {
    /// Create a new provider configuration
    #[allow(dead_code)] // Not currently used but part of public API
    pub fn new(
        provider_type: ProviderType,
        api_key: String,
        api_base: String,
        model: String,
    ) -> Self {
        Self {
            provider_type,
            api_key: SecretString::new(api_key),
            api_base,
            model,
            timeout_secs: 30,
            max_retries: 3,
            rate_limit_tpm: None,
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        // Load .env file if present (ignore errors if file doesn't exist)
        let _ = dotenvy::dotenv();

        // Determine provider type
        let provider_str = env::var("AUTOFIX_PROVIDER").unwrap_or_else(|_| "claude".to_string());
        let provider_type = ProviderType::from_str(&provider_str)?;

        // Get API key based on provider
        let api_key = match provider_type {
            ProviderType::Claude => env::var("ANTHROPIC_API_KEY")
                .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?,
            ProviderType::OpenAI => {
                env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY not set".to_string())?
            }
            ProviderType::Ollama => {
                // Ollama doesn't require an API key
                "ollama".to_string()
            }
        };

        // Get default values for this provider
        let defaults = Self::default_for_provider(provider_type);

        // Override with environment variables if present
        let api_base = env::var("AUTOFIX_API_BASE").unwrap_or(defaults.api_base);
        let model = env::var("AUTOFIX_MODEL").unwrap_or(defaults.model);
        let timeout_secs = env::var("AUTOFIX_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.timeout_secs);
        let max_retries = env::var("AUTOFIX_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.max_retries);
        let rate_limit_tpm = env::var("AUTOFIX_RATE_LIMIT_TPM")
            .ok()
            .and_then(|s| s.parse().ok())
            .or(defaults.rate_limit_tpm);

        Ok(Self {
            provider_type,
            api_key: SecretString::new(api_key),
            api_base,
            model,
            timeout_secs,
            max_retries,
            rate_limit_tpm,
        })
    }

    /// Get default configuration values for a provider
    fn default_for_provider(provider_type: ProviderType) -> Self {
        match provider_type {
            ProviderType::Claude => Self {
                provider_type,
                api_key: SecretString::new("".to_string()),
                api_base: "https://api.anthropic.com".to_string(),
                model: "claude-sonnet-4".to_string(),
                timeout_secs: 30,
                max_retries: 3,
                rate_limit_tpm: Some(30000),
            },
            ProviderType::OpenAI => Self {
                provider_type,
                api_key: SecretString::new("".to_string()),
                api_base: "https://api.openai.com/v1".to_string(),
                model: "gpt-4".to_string(),
                timeout_secs: 30,
                max_retries: 3,
                rate_limit_tpm: Some(90000),
            },
            ProviderType::Ollama => Self {
                provider_type,
                api_key: SecretString::new("ollama".to_string()),
                api_base: "http://localhost:11434/v1".to_string(),
                model: "llama2".to_string(),
                timeout_secs: 120, // Local models may be slower
                max_retries: 3,
                rate_limit_tpm: None, // No rate limit for local
            },
        }
    }

    /// Get the API key (exposed for use with clients)
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::default_for_provider(ProviderType::default())
    }
}
