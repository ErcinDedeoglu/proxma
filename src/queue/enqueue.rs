use std::collections::VecDeque;
use std::sync::Mutex;
use lazy_static::lazy_static;
use super::models::{QueueMessage, Host, Redirect};

lazy_static! {
    static ref QUEUE: Mutex<VecDeque<QueueMessage>> = Mutex::new(VecDeque::new());
}

pub struct Enqueue;

impl Enqueue {
    pub fn message(message: QueueMessage) {
        let mut queue = QUEUE.lock().unwrap();
        queue.push_back(message);
    }

    pub fn size() -> usize {
        QUEUE.lock().unwrap().len()
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
                Self::message(QueueMessage {
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
                });
            }

            // Process redirects
            if let Some(redirect_str) = labels.get("proxma.redirects") {
                for redirect in redirect_str.split(',') {
                    let parts: Vec<&str> = redirect.trim().split(':').collect();
                    if parts.len() == 2 {
                        Self::message(QueueMessage {
                            action: action.clone(),
                            container_id: container_id.clone(),
                            name: name.clone(),
                            image: image.clone(),
                            networks: networks.clone(),
                            hosting: false,
                            ssl,
                            host: None,
                            redirect: Some(Redirect {
                                from: parts[0].to_string(),
                                to: parts[1].to_string(),
                            }),
                        });
                    }
                }
            }
        }
    }
}