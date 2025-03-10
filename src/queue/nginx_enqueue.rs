use std::env;

use chrono::Utc;

use super::models::{NginxQueueMessage, Host, Redirect};
use super::shared::NGINX_QUEUE;

pub struct NginxEnqueue;

impl NginxEnqueue {
    pub fn message(mut message: NginxQueueMessage, skip_certification: bool) {
        message.created_at = Utc::now();
        message.delay_seconds = None;
        message.delay_until = None;
        message.skip_certification = skip_certification;
        NGINX_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(mut message: NginxQueueMessage, delay_for: std::time::Duration, skip_certification: bool) {
        let now = Utc::now();
        let delay_until = now + chrono::Duration::from_std(delay_for).unwrap();
        message.created_at = now;
        message.delay_seconds = Some(delay_for.as_secs());
        message.delay_until = Some(delay_until);
        message.skip_certification = skip_certification;
        NGINX_QUEUE.enqueue(message);
    }

    pub fn size() -> usize {
        NGINX_QUEUE.size()
    }

    pub fn should_process_container(labels: &std::collections::HashMap<String, String>) -> bool {
        labels.contains_key("proxma.hosts")
    }

    pub fn process_container_event(
        action: String,
        container_id: String,
        name: String,
        image: String,
        networks: Vec<String>,
        labels: std::collections::HashMap<String, String>,
    ) {
        if let Some(hosts_str) = labels.get("proxma.hosts") {
            let mut ssl_dns_provider: String = String::new();
            let mut cloudflare_email: String = String::new();
            let mut cloudflare_api_key: String = String::new();
            let mut cloudflare_api_token: String = String::new();

            let mut ssl_email: String = labels.get("proxma.ssl.email").map(|p| p.trim().to_string()).unwrap_or_default();
            if ssl_email.is_empty() {
                match env::var("PROXMA_SSL_EMAIL") {
                    Ok(env_email) => {
                        ssl_email = env_email.trim().to_string();
                    },
                    Err(_) => {
                        println!("Email missing for container {}, skipping processing", container_id);
                        return;
                    }
                }
            }

            let mut dns_provider: String = labels.get("proxma.dns.provider").map(|p| p.trim().to_string()).unwrap_or_default();
            if dns_provider.is_empty() {
                match env::var("PROXMA_DNS_PROVIDER") {
                    Ok(env_dns_provider) => {
                        dns_provider = env_dns_provider.trim().to_string().to_lowercase();
                        
                        if dns_provider != "cloudflare" && dns_provider != "route53" {
                            println!("Invalid DNS provider for container {}, skipping processing", container_id);
                            return;
                        }
                    },
                    Err(_) => {
                        println!("PROXMA_DNS_PROVIDER (proxma.dns.provider) missing for container {}, skipping processing", container_id);
                        return;
                    }
                }
            }
        
            let port = match labels.get("proxma.port").and_then(|p| p.trim().parse::<u16>().ok()) {
                Some(p) if p > 0 => p,
                _ => {
                    println!("Invalid port for container {}, skipping processing", container_id);
                    return;
                }
            };
    
            let ssl_staging: bool = labels.get("proxma.ssl.staging")
            .map(|v| v.trim().to_lowercase() == "true")
            .unwrap_or_else(|| {
                env::var("PROXMA_SSL_STAGING")
                    .map(|v| v.trim().to_lowercase() == "true")
                    .unwrap_or(true)
            });
            
            let ssl: bool = labels.get("proxma.ssl")
            .map(|v: &String| v.trim().to_lowercase() == "true")
            .unwrap_or_else(|| {
                env::var("PROXMA_SSL")
                    .map(|v| v.trim().to_lowercase() == "true")
                    .unwrap_or(false)
            });

            if ssl {
                ssl_dns_provider = labels.get("proxma.ssl.dns.provider").map(|p| p.trim().to_string()).unwrap_or_default();
                if ssl_dns_provider.is_empty() {
                    match env::var("PROXMA_SSL_DNS_PROVIDER") {
                        Ok(env_ssl_dns_provider) => {
                            ssl_dns_provider = env_ssl_dns_provider.trim().to_string().to_lowercase();
                            
                            if ssl_dns_provider != "cloudflare" && ssl_dns_provider != "cf" && ssl_dns_provider != "route53" {
                                println!("Invalid SSL DNS provider for container {}, skipping processing", container_id);
                                return;
                            }
                        },
                        Err(_) => {
                            println!("PROXMA_SSL_DNS_PROVIDER (proxma.ssl.dns.provider) missing for container {}, skipping processing", container_id);
                            return;
                        }
                    }
                }
            
                if ssl_dns_provider == "cloudflare" || ssl_dns_provider == "cf" {
                    cloudflare_email = labels.get("proxma.cloudflare.email").map(|p| p.trim().to_string()).unwrap_or_default();
                    cloudflare_api_key = labels.get("proxma.cloudflare.api_key").map(|p| p.trim().to_string()).unwrap_or_default();
                    cloudflare_api_token = labels.get("proxma.cloudflare.api_token").map(|p| p.trim().to_string()).unwrap_or_default();
                
                    if cloudflare_api_token.is_empty() {
                        if let Ok(env_api_token) = env::var("PROXMA_CLOUDFLARE_API_TOKEN") {
                            cloudflare_api_token = env_api_token.trim().to_string();
                        }
                    }
                
                    if cloudflare_api_token.is_empty() {
                        if cloudflare_api_key.is_empty() {
                            if let Ok(env_api_key) = env::var("PROXMA_CLOUDFLARE_API_KEY") {
                                cloudflare_api_key = env_api_key.trim().to_string();
                            }
                        }
                
                        if !cloudflare_api_key.is_empty() {
                            if cloudflare_email.is_empty() {
                                if let Ok(env_email) = env::var("PROXMA_CLOUDFLARE_EMAIL") {
                                    cloudflare_email = env_email.trim().to_string();
                                }
                            }
                            
                            if cloudflare_email.is_empty() {
                                println!("Email required when using Global API Key for container {}, skipping processing", container_id);
                                return;
                            }
                        }
                    }
                
                    if cloudflare_api_token.is_empty() && cloudflare_api_key.is_empty() {
                        println!("Either API Token or Global API Key must be provided for container {}, skipping processing", container_id);
                        return;
                    }
                }
            }


            let domains: Vec<String> = hosts_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
    
            for domain in domains {
                Self::message(NginxQueueMessage {
                    action: action.clone(),
                    container_id: container_id.clone(),
                    name: name.clone(),
                    image: image.clone(),
                    networks: networks.clone(),
                    hosting: true,
                    ssl,
                    host: Some(Host { domain, port }),
                    redirect: None,
                    created_at: Utc::now(),
                    delay_seconds: None,
                    delay_until: None,
                    ssl_email: ssl_email.clone(),
                    ssl_staging: ssl_staging,
                    ssl_dns_provider: Some(ssl_dns_provider.clone()),
                    cloudflare_email: Some(cloudflare_email.clone()),
                    cloudflare_api_key: Some(cloudflare_api_key.clone()),
                    cloudflare_api_token: Some(cloudflare_api_token.clone()),
                    skip_certification: false,
                }, false);
            }
    
            if let Some(redirect_str) = labels.get("proxma.redirects") {
                for redirect in redirect_str.split(',') {
                    let trimmed = redirect.trim();
                    let parts = if trimmed.contains('>') {
                        trimmed.split('>').collect::<Vec<&str>>()
                    } else {
                        trimmed.split(':').collect::<Vec<&str>>()
                    };
            
                    if parts.len() == 2 {
                        Self::message(NginxQueueMessage {
                            action: action.clone(),
                            container_id: container_id.clone(),
                            name: name.clone(),
                            image: image.clone(),
                            networks: networks.clone(),
                            hosting: false,
                            ssl,
                            host: None,
                            redirect: Some(Redirect {
                                from: parts[0].trim().to_string(),
                                to: parts[1].trim().to_string(),
                            }),
                            delay_until: None,
                            created_at: Utc::now(),
                            delay_seconds: None,
                            ssl_email: ssl_email.clone(),
                            ssl_staging: ssl_staging,
                            ssl_dns_provider: Some(ssl_dns_provider.clone()),
                            cloudflare_email: Some(cloudflare_email.clone()),
                            cloudflare_api_key: Some(cloudflare_api_key.clone()),
                            cloudflare_api_token: Some(cloudflare_api_token.clone()),
                            skip_certification: false,
                        }, false);
                    }
                }
            }
        }
    }
}