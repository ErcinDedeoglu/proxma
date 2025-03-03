use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRule {
    pub id: String,
    pub domains: Vec<String>,
    pub upstream: String,
    pub redirects: Vec<(String, String)>,
}

impl Default for ProxyRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            domains: Vec::new(),
            upstream: String::new(),
            redirects: Vec::new(),
        }
    }
}