//! Cache configuration models for Nginx

/// Configuration for Nginx caching
#[derive(Debug, Clone)]
pub struct Cache {
    /// Whether caching is enabled
    pub enabled: bool,
    
    /// Maximum size of the cache (e.g., "500m", "2g")
    pub size: String,
    
    /// Size of the shared memory zone in MB (e.g., "10", "512")
    pub memory: String,
    
    /// How long to cache valid responses (e.g., "60m", "1h", "7d")
    pub ttl: String,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            enabled: false,
            size: "500m".to_string(),
            memory: "10".to_string(),
            ttl: "60m".to_string(),
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
            ..Default::default()
        }
    }
    
    /// Builder method to set size
    pub fn with_max_size(mut self, size: &str) -> Self {
        self.size = size.to_string();
        self
    }
    
    /// Builder method to set memory_size
    pub fn with_memory_size(mut self, memory: &str) -> Self {
        self.memory = memory.to_string();
        self
    }
    
    /// Builder method to set valid_duration
    pub fn with_valid_duration(mut self, ttl: &str) -> Self {
        self.ttl = ttl.to_string();
        self
    }
}