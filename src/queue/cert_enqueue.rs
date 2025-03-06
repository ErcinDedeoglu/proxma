use chrono::Utc;
use super::models::CertificateQueueMessage;
use super::shared::CERT_QUEUE;

pub struct CertEnqueue;

impl CertEnqueue {
    pub fn message(domain: String) {
        let message = CertificateQueueMessage {
            domain,
            created_at: Utc::now(),
            delay_seconds: None,
            delay_until: None,
        };
        CERT_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(domain: String, delay_for: std::time::Duration) {
        let now = Utc::now();
        let delay_until = now + chrono::Duration::from_std(delay_for).unwrap();
        let message = CertificateQueueMessage {
            domain,
            created_at: now,
            delay_seconds: Some(delay_for.as_secs()),
            delay_until: Some(delay_until),
        };
        CERT_QUEUE.enqueue(message);
    }
    
    pub fn direct_message(message: CertificateQueueMessage) {
        CERT_QUEUE.enqueue(message);
    }
    
    pub fn size() -> usize {
        CERT_QUEUE.size()
    }
}