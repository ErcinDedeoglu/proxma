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

    /// Only clear pending messages for the *same* container_id and domain.
    pub fn clear_pending_messages(domain: &str, container_id: &str) {
        let mut removed = false;
        loop {
            if let Some(nginx_msg) = Self::peek() {
                let is_host_match = nginx_msg.host.as_ref().map(|h| h.domain == domain).unwrap_or(false);
                let is_redirect_match = nginx_msg.redirect.as_ref().map(|r| r.from == domain).unwrap_or(false);

                // Only remove if both domain and container_id match
                if (is_host_match || is_redirect_match) && nginx_msg.container_id == container_id {
                    Self::message();
                    println!("🧹 Removed pending Nginx queue message for '{}' (container_id: {})", domain, container_id);
                    removed = true;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !removed {
            println!("🧹 No pending Nginx queue messages found for '{}' with container_id '{}'", domain, container_id);
        }
    }
}