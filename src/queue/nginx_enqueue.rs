use std::env;

use chrono::Utc;

use super::models::{NginxQueueMessage, Host, Redirect};
use super::shared::NGINX_QUEUE;

pub struct NginxEnqueue;

impl NginxEnqueue {
    pub fn message(mut message: NginxQueueMessage) {
        message.created_at = Utc::now();
        message.delay_seconds = None;
        message.delay_until = None;
        NGINX_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(mut message: NginxQueueMessage, delay_for: std::time::Duration) {
        let now = Utc::now();
        let delay_until = now + chrono::Duration::from_std(delay_for).unwrap();
        message.created_at = now;
        message.delay_seconds = Some(delay_for.as_secs());
        message.delay_until = Some(delay_until);
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
                });
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
                        });
                    }
                }
            }
        }
    }
}