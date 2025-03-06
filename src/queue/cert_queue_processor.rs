use super::CertDequeue;
use super::models::CertificateQueueMessage;
use crate::certbot::Certbot;
use lazy_static::lazy_static;


lazy_static! {
    pub static ref CERTBOT: Certbot = Certbot::new("/var/www/html", "dublokcom@gmail.com")
        .agree_tos(true)
        .staging(true)
        .no_eff_email(false)
        .config_dir("/var/proxma/letsencrypt")
        .work_dir("/var/proxma/letsencrypt/work")
        .logs_dir("/var/proxma/logs");
}

pub struct CertQueueProcessor;

impl CertQueueProcessor {
    async fn process_certificate_request(message: &CertificateQueueMessage) {
        println!("🔒 Processing certificate request for domain: {}", message.domain);
        
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
    
    pub async fn start() {
        loop {
            if !CertDequeue::is_empty() {
                if let Some(message) = CertDequeue::message() {
                    println!("📨 [CertQueueProcessor] Processing message - domain: {}", message.domain);
                    Self::process_certificate_request(&message).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}