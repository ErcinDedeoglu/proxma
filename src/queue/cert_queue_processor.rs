use super::CertDequeue;
use super::models::CertificateQueueMessage;
use crate::{certbot::{Certbot, CertificateRequestResult}, queue::CertEnqueue};
use chrono::Utc;
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
        
        match CERTBOT.check_acme_challenge(&message.domain) {
            Ok(CertificateRequestResult::Success) => {
                println!("✅ ACME challenge verification successful for: {}", message.domain);
                
                match CERTBOT.request_certificate(&message.domain) {
                    Ok(_) => {
                        println!("✅ Certificate request successful for '{}'", message.domain);
                    },
                    Err(e) => {
                        eprintln!("❌ Certificate request failed for '{}': {}", message.domain, e);
                    }
                }
            },
            Ok(CertificateRequestResult::AcmeChallengeFailure(error)) => {
                eprintln!("❌ ACME challenge failed for '{}': {}", message.domain, error);
            },
            Ok(CertificateRequestResult::CertbotError(error)) => {
                eprintln!("❌ Certbot error for '{}': {}", message.domain, error);
            },
            Err(e) => {
                eprintln!("❌ ACME challenge check failed for '{}': {}", message.domain, e);
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    pub async fn start() {
        loop {
            if !CertDequeue::is_empty() {
                if let Some(message) = CertDequeue::message() {
                    // Check if the message is ready to be processed
                    let now = Utc::now();
                    if let Some(delay_until) = message.delay_until {
                        if now < delay_until {
                            // Message is not ready yet, put it back in the queue
                            CertEnqueue::direct_message(message);
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                    }
                    
                    // Process the message as normal
                    println!("📨 [CertQueueProcessor] Processing message - domain: {}", message.domain);
                    Self::process_certificate_request(&message).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}