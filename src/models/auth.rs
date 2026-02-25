use serde::{Deserialize, Serialize};

/// Authentication type for proxied services
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AuthType {
    /// HTTP Basic authentication (username/password)
    Basic,
    /// Bearer token authentication (static token)
    Bearer,
    /// Accept either Basic or Bearer authentication
    Both,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Basic
    }
}

impl AuthType {
    /// Parse auth type from string label value
    pub fn from_str_label(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "bearer" => AuthType::Bearer,
            "both" => AuthType::Both,
            _ => AuthType::Basic,
        }
    }
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::Basic => write!(f, "basic"),
            AuthType::Bearer => write!(f, "bearer"),
            AuthType::Both => write!(f, "both"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    /// Whether authentication is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Authentication type: basic, bearer, or both
    #[serde(default)]
    pub auth_type: AuthType,
    /// Authentication realm (displayed in browser prompt for basic auth)
    #[serde(default = "default_realm")]
    pub realm: String,
    /// Username for basic auth
    #[serde(default = "default_username")]
    pub username: String,
    /// Password for basic auth (will be hashed)
    #[serde(default = "default_password")]
    pub password: String,
    /// Static bearer token for token-based auth
    #[serde(default)]
    pub token: String,
}

fn default_realm() -> String { "Restricted Area".to_string() }
fn default_username() -> String { "root".to_string() }
fn default_password() -> String { "root".to_string() }

impl Default for Auth {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_type: AuthType::default(),
            realm: default_realm(),
            username: default_username(),
            password: default_password(),
            token: String::new(),
        }
    }
}