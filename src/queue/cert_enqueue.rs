use chrono::Utc;
use super::models::{CertificateQueueMessage, NginxQueueMessage};
use super::shared::CERT_QUEUE;

pub struct CertEnqueue;

impl CertEnqueue {
    pub fn message(domain: String, nginx_queue_message: NginxQueueMessage) {
        let ssl_email = nginx_queue_message.ssl_email.clone();
        let ssl_staging = nginx_queue_message.ssl_staging.clone();
        let ssl_dns_provider = nginx_queue_message.ssl_dns_provider.clone();
        let cloudflare_email = nginx_queue_message.cloudflare_email.clone();
        let cloudflare_api_key = nginx_queue_message.cloudflare_api_key.clone();
        let cloudflare_api_token = nginx_queue_message.cloudflare_api_token.clone();
        let message = CertificateQueueMessage {
            domain,
            created_at: Utc::now(),
            delay_seconds: None,
            delay_until: None,
            nginx_queue_message,
            ssl_email: ssl_email.clone(),
            ssl_staging: ssl_staging.clone(),
            ssl_dns_provider: ssl_dns_provider.clone(),
            cloudflare_email: cloudflare_email.clone(),
            cloudflare_api_key: cloudflare_api_key.clone(),
            cloudflare_api_token: cloudflare_api_token.clone(),
        };
        CERT_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(domain: String, delay_for: std::time::Duration, nginx_queue_message: NginxQueueMessage) {
        let now = Utc::now();
        let delay_until = now + chrono::Duration::from_std(delay_for).unwrap();
        let ssl_email = nginx_queue_message.ssl_email.clone();
        let ssl_staging = nginx_queue_message.ssl_staging.clone();
        let ssl_dns_provider = nginx_queue_message.ssl_dns_provider.clone();
        let cloudflare_email = nginx_queue_message.cloudflare_email.clone();
        let cloudflare_api_key = nginx_queue_message.cloudflare_api_key.clone();
        let cloudflare_api_token = nginx_queue_message.cloudflare_api_token.clone();
        let message = CertificateQueueMessage {
            domain,
            created_at: now,
            delay_seconds: Some(delay_for.as_secs()),
            delay_until: Some(delay_until),
            nginx_queue_message,
            ssl_email: ssl_email.clone(),
            ssl_staging: ssl_staging.clone(),
            ssl_dns_provider: ssl_dns_provider.clone(),
            cloudflare_email: cloudflare_email.clone(),
            cloudflare_api_key: cloudflare_api_key.clone(),
            cloudflare_api_token: cloudflare_api_token.clone()
        };
        CERT_QUEUE.enqueue(message);
    }
    
    pub fn direct_message(message: CertificateQueueMessage) {
        CERT_QUEUE.enqueue(message);
    }
}