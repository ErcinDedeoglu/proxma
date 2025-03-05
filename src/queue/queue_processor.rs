use super::Dequeue;
use tokio::time::sleep;
use std::time::Duration;
use super::models::{Host, Redirect};
use crate::nginx::NginxManager;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref NGINX_MANAGER: NginxManager = NginxManager::new("/etc/nginx/conf.d", "/var/www/html");
}

pub struct QueueProcessor;

impl QueueProcessor {
    // New consolidated method for "start" actions
    pub async fn process_start_action(host: Option<&Host>, redirect: Option<&Redirect>, ssl: bool) {
        if let Some(host) = host {
            println!("🏠 Configuring host: {} (SSL: {})", host.domain, ssl);
            
            
            sleep(Duration::from_millis(1000)).await;
        }
        
        if let Some(redirect) = redirect {
            println!("➡️ Configuring redirect: {} -> {}", redirect.from, redirect.to);
            
            
            sleep(Duration::from_millis(1000)).await;
        }
    }

    // New consolidated method for "die" actions
    pub async fn process_die_action(host: Option<&Host>, redirect: Option<&Redirect>) {
        if let Some(host) = host {
            println!("🏠 Removing host: {}", host.domain);
            // Uncomment when needed:
            // NGINX_MANAGER.remove_host(host);
            sleep(Duration::from_millis(1000)).await;
        }
        
        if let Some(redirect) = redirect {
            println!("➡️ Removing redirect: {}", redirect.from);
            // Uncomment when needed:
            // NGINX_MANAGER.remove_redirect(redirect);
            sleep(Duration::from_millis(1000)).await;
        }
    }

    // You could also simplify your start method to use the consolidated methods directly
    pub async fn start() {
        loop {
            if !Dequeue::is_empty() {
                if let Some(message) = Dequeue::message() {
                    println!("Processing message - hosting: {}, has_host: {}, has_redirect: {}",
                        message.hosting,
                        message.host.is_some(),
                        message.redirect.is_some()
                    );
                    
                    match message.action.as_str() {
                        "start" => {
                            Self::process_start_action(message.host.as_ref(), message.redirect.as_ref(), message.ssl).await;
                        },
                        "die" => {
                            Self::process_die_action(message.host.as_ref(), message.redirect.as_ref()).await;
                        }, _ => println!("Unknown action: {}", message.action)
                    }
                }
            }

            sleep(Duration::from_millis(1000)).await;
        }
    }
}