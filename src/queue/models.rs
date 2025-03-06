use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct NginxQueueMessage {
    pub action: String,
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub networks: Vec<String>,
    pub hosting: bool,
    pub ssl: bool,
    pub host: Option<Host>,
    pub redirect: Option<Redirect>,
    pub delay_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delay_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Host {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct CertificateQueueMessage {
    pub domain: String,
    pub delay_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delay_seconds: Option<u64>,
}