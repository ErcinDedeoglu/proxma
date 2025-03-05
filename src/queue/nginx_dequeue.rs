use super::models::NginxQueueMessage;
use super::shared::NGINX_QUEUE;

pub struct NginxDequeue;

impl NginxDequeue {
    pub fn message() -> Option<NginxQueueMessage> {
        NGINX_QUEUE.dequeue()
    }
    
    pub fn size() -> usize {
        NGINX_QUEUE.size()
    }
    
    pub fn peek() -> Option<NginxQueueMessage> {
        NGINX_QUEUE.peek()
    }
    
    pub fn is_empty() -> bool {
        NGINX_QUEUE.is_empty()
    }
}