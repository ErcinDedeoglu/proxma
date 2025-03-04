use std::collections::VecDeque;
use std::sync::Mutex;
use lazy_static::lazy_static;
use super::models::QueueMessage;

lazy_static! {
    pub(crate) static ref QUEUE: Mutex<VecDeque<QueueMessage>> = Mutex::new(VecDeque::new());
}