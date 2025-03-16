use chrono::{DateTime, Utc};

use crate::models::{Auth, Webserver};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NginxQueueMessage {
    pub action: String,
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub networks: Vec<String>,
    pub hosting: bool,
    pub ssl: bool,
    pub ssl_email: String,
    pub host: Option<Host>,
    pub redirect: Option<Redirect>,
    pub delay_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delay_seconds: Option<u64>,
    pub ssl_staging: bool,
    pub dns_provider: Option<String>,
    pub ssl_dns_provider: Option<String>,
    pub cloudflare_email: Option<String>,
    pub cloudflare_api_key: Option<String>,
    pub cloudflare_api_token: Option<String>,
    pub skip_certification: bool,
    pub skip_dns: bool,
    pub dns_record_type: Option<String>,
    pub dns_record_proxied: Option<bool>,
    pub dns_record_target: Option<String>,
    pub auth: Auth,
    pub webserver: Webserver,
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
#[allow(dead_code)]
pub struct CertificateQueueMessage {
    pub nginx_queue_message: NginxQueueMessage,
    pub domain: String,
    pub delay_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub delay_seconds: Option<u64>,
    pub ssl_email: String,
    pub ssl_staging: bool,
    pub ssl_dns_provider: Option<String>,
    pub cloudflare_email: Option<String>,
    pub cloudflare_api_key: Option<String>,
    pub cloudflare_api_token: Option<String>,
}