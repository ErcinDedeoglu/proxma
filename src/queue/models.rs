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
}