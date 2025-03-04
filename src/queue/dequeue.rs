use super::models::QueueMessage;
use super::shared::QUEUE;

pub struct Dequeue;

impl Dequeue {
    pub fn message() -> Option<QueueMessage> {
        let mut queue = QUEUE.lock().unwrap();
        queue.pop_front()
    }

    pub fn size() -> usize {
        QUEUE.lock().unwrap().len()
    }

    // Helper method to peek without removing
    pub fn peek() -> Option<QueueMessage> {
        let queue = QUEUE.lock().unwrap();
        queue.front().cloned()
    }

    // Helper method to check if queue is empty
    pub fn is_empty() -> bool {
        let queue = QUEUE.lock().unwrap();
        queue.is_empty()
    }
}