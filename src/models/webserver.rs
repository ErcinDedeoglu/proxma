use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webserver {
    /// Authentication realm (displayed in browser prompt)
    #[serde(default = "default_body_size")]
    pub body_size: String, // m for megabytes, k for kilobytes, g for gigabytes, usage: 1m, 1k, 1g
}

fn default_body_size() -> String { "100m".to_string() }

impl Default for Webserver {
    fn default() -> Self {
        Self {
            body_size: default_body_size()
        }
    }
}