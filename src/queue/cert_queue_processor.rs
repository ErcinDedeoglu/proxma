use super::CertDequeue;
use super::models::CertificateQueueMessage;
use crate::{certbot::{Certbot, CertificateRequestResult, ChallengeType, DnsCredentials}, queue::{CertEnqueue, NginxEnqueue}};
use chrono::Utc;
use lazy_static::lazy_static;
use crate::acme_helper::check_acme_challenge;


lazy_static! {
    pub static ref CERTBOT: Certbot = Certbot::new();
}

pub struct CertQueueProcessor;

impl CertQueueProcessor {
    fn requeue_certification_request(message: &CertificateQueueMessage) {
        let delay_in_seconds = 600 + message.delay_seconds.unwrap_or(0);
        let delay_in_seconds = if delay_in_seconds >= 3600 { 3600 } else { delay_in_seconds };
        let delay: std::time::Duration = std::time::Duration::from_secs(delay_in_seconds);
        CertEnqueue::message_with_delay(message.domain.clone(), delay, message.nginx_queue_message.clone());
    }

    async fn process_certificate_request(message: &CertificateQueueMessage) {
        println!("🔒 Processing certificate request for domain: {}", message.domain);
        
        if message.ssl_dns_provider.as_deref() == Some("cloudflare") {
            let credentials = DnsCredentials {
                provider: message.ssl_dns_provider.clone().unwrap_or_default(),
                api_token: message.ssl_dns_api_token.clone().unwrap_or_default(),
                email: Some(message.ssl_dns_email.clone().unwrap_or_default()),
                api_key: Some(message.ssl_dns_api_key.clone().unwrap_or_default()),
            };
            let challenge_type = ChallengeType::Dns(
                "cloudflare".to_string(),
                Some(credentials)
            );
            
            match CERTBOT.request_certificate(&message.domain, &message.ssl_email, challenge_type, message.ssl_staging) {
                Ok(_) => {
                    println!("✅ Certificate request successful for '{}'", message.domain);
                    NginxEnqueue::message(message.nginx_queue_message.clone(), true);
                },
                Err(e) => {
                    eprintln!("❌ Certificate request failed for '{}': {}", message.domain, e);
                    Self::requeue_certification_request(message);
                }
            }
        } else {
            match check_acme_challenge(&message.domain) {
                Ok(CertificateRequestResult::Success) => {
                    println!("✅ ACME challenge verification successful for: {}", message.domain);
                    
                    match CERTBOT.request_certificate(&message.domain, &message.ssl_email, ChallengeType::Webroot, message.ssl_staging) {
                        Ok(_) => {
                            println!("✅ Certificate request successful for '{}'", message.domain);
                            NginxEnqueue::message(message.nginx_queue_message.clone(), true);
                        },
                        Err(e) => {
                            eprintln!("❌ Certificate request failed for '{}': {}", message.domain, e);
                            Self::requeue_certification_request(message);
                        }
                    }
                },
                Ok(CertificateRequestResult::AcmeChallengeFailure(error)) => {
                    eprintln!("❌ ACME challenge failed for '{}': {}", message.domain, error);
                    let delay_in_seconds = 10 + message.delay_seconds.unwrap_or(0);
                    let delay_in_seconds = if delay_in_seconds >= 3600 { 3600 } else { delay_in_seconds };
                    let delay: std::time::Duration = std::time::Duration::from_secs(delay_in_seconds);
                    CertEnqueue::message_with_delay(message.domain.clone(), delay, message.nginx_queue_message.clone());
                    eprintln!("🔁 Re-enqueued message for '{}'", message.domain);
                },
                Ok(CertificateRequestResult::CertbotError(error)) => {
                    eprintln!("❌ Certbot error for '{}': {}", message.domain, error);
                },
                Err(e) => {
                    eprintln!("❌ ACME challenge check failed for '{}': {}", message.domain, e);
                }
            }
        }
    }
    
    pub async fn start() {
        loop {
            if !CertDequeue::is_empty() {
                if let Some(message) = CertDequeue::message() {
                    // Check if the message is ready to be processed
                    let now = Utc::now();
                    if let Some(delay_until) = message.delay_until {
                        if now < delay_until {
                            CertEnqueue::direct_message(message.clone());
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                    }
                    
                    // Process the message as normal
                    println!("📨 [CertQueueProcessor] Processing message - domain: {}", message.domain);
                    Self::process_certificate_request(&message).await;
                }
            }
            else {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }
    }
}