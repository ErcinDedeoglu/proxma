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

impl Cache {
    /// Create a new enabled cache configuration with default settings
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }
    
    /// Create a new cache configuration with custom settings
    pub fn new(
        size: &str,
        memory: &str,
        ttl: &str,
    ) -> Self {
        Self {
            enabled: true,
            size: size.to_string(),
            memory: memory.to_string(),
            ttl: ttl.to_string(),
        }
    }
    
    /// Builder method to set size
    pub fn with_size(mut self, size: &str) -> Self {
        self.size = size.to_string();
        self
    }
    
    /// Builder method to set memory
    pub fn with_memory(mut self, memory: &str) -> Self {
        self.memory = memory.to_string();
        self
    }
    
    /// Builder method to set ttl
    pub fn with_ttl(mut self, ttl: &str) -> Self {
        self.ttl = ttl.to_string();
        self
    }
    
    /// Update specific fields, keeping defaults for others
    pub fn update(&mut self, enabled: Option<bool>, size: Option<&str>, memory: Option<&str>, ttl: Option<&str>) {
        if let Some(enabled) = enabled {
            self.enabled = enabled;
        }
        
        if let Some(size) = size {
            self.size = size.to_string();
        }
        
        if let Some(memory) = memory {
            self.memory = memory.to_string();
        }
        
        if let Some(ttl) = ttl {
            self.ttl = ttl.to_string();
        }
    }
}