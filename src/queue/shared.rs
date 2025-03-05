use lazy_static::lazy_static;
use super::models::{NginxQueueMessage, CertificateQueueMessage};
use super::generic_queue::GenericQueue;

lazy_static! {
    pub static ref NGINX_QUEUE: GenericQueue<NginxQueueMessage> = GenericQueue::new();
    pub static ref CERT_QUEUE: GenericQueue<CertificateQueueMessage> = GenericQueue::new();
}