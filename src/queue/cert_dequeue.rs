use super::models::CertificateQueueMessage;
use super::shared::CERT_QUEUE;

pub struct CertDequeue;

impl CertDequeue {
    pub fn message() -> Option<CertificateQueueMessage> {
        CERT_QUEUE.dequeue()
    }
    
    pub fn peek() -> Option<CertificateQueueMessage> {
        CERT_QUEUE.peek()
    }
    
    pub fn is_empty() -> bool {
        CERT_QUEUE.is_empty()
    }
    
    pub fn size() -> usize {
        CERT_QUEUE.size()
    }

    pub fn clear_pending_messages(domain: &str) {
        while let Some(cert_msg) = Self::peek() {
            if cert_msg.domain == domain {
                Self::message();
                println!("🧹 Removed pending certificate request for '{}'", domain);
            } else {
                break;
            }
        }
    }
}