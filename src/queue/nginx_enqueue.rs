use chrono::Utc;

use super::models::{NginxQueueMessage, Host, Redirect};
use super::shared::NGINX_QUEUE;

pub struct NginxEnqueue;

impl NginxEnqueue {
    pub fn message(message: NginxQueueMessage) {
        NGINX_QUEUE.enqueue(message);
    }

    pub fn message_with_delay(mut message: NginxQueueMessage, delay_for: std::time::Duration) {
        let delay_until = Utc::now() + chrono::Duration::from_std(delay_for).unwrap();
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
            let domains: Vec<String> = hosts_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            let port = labels.get("proxma.port")
                .map(|p| p.trim().parse::<u16>().unwrap_or(80))
                .unwrap_or(80);
                
            let ssl = labels.get("proxma.ssl")
                .map(|v| v.trim().to_lowercase() == "true")
                .unwrap_or(false);

            // Process domains
            for domain in domains {
                Self::message(NginxQueueMessage {
                    action: action.clone(),
                    container_id: container_id.clone(),
                    name: name.clone(),
                    image: image.clone(),
                    networks: networks.clone(),
                    hosting: true,
                    ssl,
                    host: Some(Host {
                        domain,
                        port,
                    }),
                    redirect: None,
                    delay_until: None,
                });
            }

            // Process redirects
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
                        });
                    }
                }
            }
        }
    }
}