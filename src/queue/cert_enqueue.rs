use chrono::Utc;
use super::models::{CertificateQueueMessage, NginxQueueMessage};
use super::shared::CERT_QUEUE;

pub struct CertEnqueue;

impl CertEnqueue {
    pub fn message(domain: String, nginx_queue_message: NginxQueueMessage) {
        let message = CertificateQueueMessage {
            domain,
            created_at: Utc::now(),
            delay_seconds: None,
            delay_until: None,
            nginx_queue_message
        };
        CERT_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(domain: String, delay_for: std::time::Duration, nginx_queue_message: NginxQueueMessage) {
        let now = Utc::now();
        let delay_until = now + chrono::Duration::from_std(delay_for).unwrap();
        let message = CertificateQueueMessage {
            domain,
            created_at: now,
            delay_seconds: Some(delay_for.as_secs()),
            delay_until: Some(delay_until),
            nginx_queue_message
        };
        CERT_QUEUE.enqueue(message);
    }
    
    pub fn direct_message(message: CertificateQueueMessage) {
        CERT_QUEUE.enqueue(message);
    }
}