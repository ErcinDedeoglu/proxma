use super::Dequeue;
use tokio::time::sleep;
use std::time::Duration;
use super::models::{Host, Redirect};

pub struct QueueProcessor;

impl QueueProcessor {
    pub async fn process_host(host: &Host, ssl: bool, action: &str) {
        match action {
            "start" => {
                println!("🏠 Configuring host: {} (SSL: {})", host.domain, ssl);
                // nginx::configure_host(host, ssl)
            },
            "die" => {
                println!("🏠 Removing host: {}", host.domain);
                // nginx::remove_host(host)
            },
            _ => {}
        }
    }

    pub async fn process_redirect(redirect: &Redirect, action: &str) {
        match action {
            "start" => {
                println!("➡️ Configuring redirect: {} -> {}", redirect.from, redirect.to);
                // nginx::configure_redirect(redirect)
            },
            "die" => {
                println!("➡️ Removing redirect: {}", redirect.from);
                // nginx::remove_redirect(redirect)
            },
            _ => {}
        }
    }

    pub async fn start() {
        loop {
            if !Dequeue::is_empty() {
                if let Some(message) = Dequeue::message() {
                    println!("Processing message - hosting: {}, has_host: {}, has_redirect: {}", 
                        message.hosting,
                        message.host.is_some(),
                        message.redirect.is_some()
                    );

                    if let Some(host) = &message.host {
                        Self::process_host(host, message.ssl, &message.action).await;
                    }
                    
                    if let Some(redirect) = &message.redirect {
                        Self::process_redirect(redirect, &message.action).await;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}