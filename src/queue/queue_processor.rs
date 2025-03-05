use super::Dequeue;
use super::models::{Host, QueueMessage, Redirect};
use crate::nginx::NginxManager;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref NGINX_MANAGER: NginxManager =
        NginxManager::new("/etc/nginx/conf.d", "/var/www/html");
}

pub struct QueueProcessor;

impl QueueProcessor {
    pub async fn process_start_action(message: &QueueMessage) {
        if let Some(host) = &message.host {
            println!("🏠 Configuring host: {} (SSL: {})", host.domain, message.ssl);
            let upstream_url = format!("http://{}:{}", message.name, host.port);
            match NGINX_MANAGER.add_container_host_rule(&host.domain, &upstream_url, message.ssl) {
                Ok(_) => {
                    match NGINX_MANAGER.validate_nginx_config() {
                        Ok(_) => match NGINX_MANAGER.reload_nginx() {
                            Ok(_) => println!("✅ Nginx configuration added and nginx reloaded."),
                            Err(e) => eprintln!("❌ Failed to reload nginx: {}", e),
                        },
                        Err(e) => {
                            eprintln!("❌ Nginx configuration validation failed: {}", e);
                            if let Err(remove_err) = NGINX_MANAGER.remove_rule(&host.domain) {
                                eprintln!("❌ Failed to remove invalid configuration: {}", remove_err);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("❌ Error adding nginx config for '{}': {}", host.domain, e),
            }
        } else if let Some(redirect) = &message.redirect {
            println!("➡️ Configuring redirect: {} -> {}", redirect.from, redirect.to);
            // Add your redirect configuration logic here
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    pub async fn process_die_action(message: &QueueMessage) {
        if let Some(host) = &message.host {
            println!("🏠 Removing host: {}", host.domain);
            // Implement removal logic here
        } else if let Some(redirect) = &message.redirect {
            println!("➡️ Removing redirect: {}", redirect.from);
            // Remove your redirect configuration logic here
        }

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }

    pub async fn start() {
        loop {
            if !Dequeue::is_empty() {
                if let Some(message) = Dequeue::message() {
                    println!(
                        "📨 Processing message - hosting: {}, has_host: {}, has_redirect: {}",
                        message.hosting,
                        message.host.is_some(),
                        message.redirect.is_some()
                    );

                    match message.action.as_str() {
                        "start" => Self::process_start_action(&message).await,
                        "die" => Self::process_die_action(&message).await,
                        _ => eprintln!("⚠️ Unknown action: '{}'", message.action),
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}