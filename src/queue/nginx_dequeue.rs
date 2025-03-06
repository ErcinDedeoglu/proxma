use super::models::NginxQueueMessage;
use super::shared::NGINX_QUEUE;

pub struct NginxDequeue;

impl NginxDequeue {
    pub fn message() -> Option<NginxQueueMessage> {
        NGINX_QUEUE.dequeue()
    }
    
    pub fn peek() -> Option<NginxQueueMessage> {
        NGINX_QUEUE.peek()
    }
    
    pub fn is_empty() -> bool {
        NGINX_QUEUE.is_empty()
    }

    pub fn clear_pending_messages(domain: &str) {
        while let Some(nginx_msg) = Self::peek() {
            let should_remove = match (&nginx_msg.host, &nginx_msg.redirect) {
                (Some(host), _) => host.domain == domain,
                (_, Some(redirect)) => redirect.from == domain,
                (None, None) => false,
            };

            if should_remove {
                Self::message();
                println!("🧹 Removed pending Nginx queue message for '{}'", domain);
            } else {
                break;
            }
        }
    }
}