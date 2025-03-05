use chrono::Utc;
use super::models::CertificateQueueMessage;
use super::shared::CERT_QUEUE;

pub struct CertEnqueue;

impl CertEnqueue {
    pub fn message(domain: String) {
        let message = CertificateQueueMessage {
            domain,
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