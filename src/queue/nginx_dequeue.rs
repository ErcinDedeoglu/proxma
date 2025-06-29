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

    /// Only clear pending messages for the *same* container_id, domain, and action type.
    /// This prevents clearing "start" events when processing "die" events during restarts.
    pub fn clear_pending_messages(domain: &str, container_id: &str) {
        Self::clear_pending_messages_by_action(domain, container_id, "die");
    }

    /// Clear pending messages for specific action type to avoid clearing restart events
    pub fn clear_pending_messages_by_action(domain: &str, container_id: &str, action: &str) {
        let mut removed = false;
        let mut messages_to_requeue = Vec::new();
        
        // First, collect all messages and separate the ones we want to keep
        while let Some(nginx_msg) = Self::message() {
            let is_host_match = nginx_msg.host.as_ref().map(|h| h.domain == domain).unwrap_or(false);
            let is_redirect_match = nginx_msg.redirect.as_ref().map(|r| r.from == domain).unwrap_or(false);

            // Only remove if domain, container_id, AND action match
            if (is_host_match || is_redirect_match) && nginx_msg.container_id == container_id && nginx_msg.action == action {
                println!("🧹 Removed pending Nginx queue message for '{}' (container_id: {}, action: {})", domain, container_id, action);
                removed = true;
            } else {
                // Keep this message - we'll requeue it
                messages_to_requeue.push(nginx_msg);
            }
        }
        
        // Requeue the messages we want to keep
        for msg in messages_to_requeue {
            use super::shared::NGINX_QUEUE;
            NGINX_QUEUE.enqueue(msg);
        }
        
        if !removed {
            println!("🧹 No pending Nginx queue messages found for '{}' with container_id '{}' and action '{}'", domain, container_id, action);
        }
    }
}