//! Cache configuration models for Nginx
use serde::{Deserialize, Serialize};

/// Configuration for Nginx caching
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cache {
    /// Whether caching is enabled
    #[serde(default)]
    pub enabled: bool,
    
    /// Maximum size of the cache (e.g., "500m", "2g")
    #[serde(default = "default_size")]
    pub size: String,
    
    /// Size of the shared memory zone in MB (e.g., "10", "512")
    #[serde(default = "default_memory")]
    pub memory: String,
    
    /// How long to cache valid responses (e.g., "60m", "1h", "7d")
    #[serde(default = "default_ttl")]
    pub ttl: String,
}

fn default_size() -> String { "500m".to_string() }
fn default_memory() -> String { "10".to_string() }
fn default_ttl() -> String { "60m".to_string() }

impl Default for Cache {
    fn default() -> Self {
        Self {
            enabled: false,
            size: default_size(),
            memory: default_memory(),
            ttl: default_ttl(),
        }
    }
}