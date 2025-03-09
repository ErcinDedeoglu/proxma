use super::NginxDequeue;
use super::models::NginxQueueMessage;
use crate::nginx::NginxManager;
use crate::queue::cert_enqueue::CertEnqueue;
use crate::queue::{CertDequeue, NginxEnqueue};
use chrono::Utc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref NGINX_MANAGER: NginxManager =
        NginxManager::new("/etc/nginx/conf.d", "/var/www/html");
}

pub struct NginxQueueProcessor;

impl NginxQueueProcessor {
    pub async fn process_start_action(message: &NginxQueueMessage) {
        if let Some(host) = &message.host {
            println!("🏠 Configuring host: {} (SSL: {})", host.domain, message.ssl);
            let upstream_url = format!("http://{}:{}", message.name, host.port);
            let use_ssl = message.ssl && NGINX_MANAGER.check_ssl_certificates_exist(&host.domain, message.ssl_staging);
            
            if use_ssl {
                println!("🔒 SSL certificate found for '{}'", host.domain);
            } else {
                println!("🔒 SSL certificate not found for '{}'", host.domain);
            }
            
            match NGINX_MANAGER.add_container_host_rule(&host.domain, &upstream_url, use_ssl, message.ssl_staging) {
                Ok(_) => {
                    println!("📝 Created Nginx config for '{}', proxy to '{}'", host.domain, upstream_url);
                    
                    match NGINX_MANAGER.validate_nginx_config() {
                        Ok(_) => {
                            println!("✅ Nginx configuration is valid");
                            match NGINX_MANAGER.reload_nginx() {
                                Ok(_) => {
                                    println!("✅ Nginx configuration reloaded successfully");
                                    if !message.skip_certification {
                                        CertEnqueue::message(host.domain.clone(), message.clone());
                                        print!("🔒 Certificate request queued for '{}'", host.domain);
                                    }
                                    else {
                                        println!("🔒 Skipping certificate request for '{}'", host.domain);
                                    }
                                },
                                Err(e) => eprintln!("❌ Failed to reload nginx: {}", e),
                            }
                        },
                        Err(e) => {
                            eprintln!("❌ Nginx configuration validation failed:\n{}", e);
                            match NGINX_MANAGER.remove_rule(&host.domain) {
                                Ok(_) => println!("✅ Invalid configuration removed successfully"),
                                Err(e) => eprintln!("❌ Failed to remove invalid configuration: {}", e),
                            }
                        }
                    }
                }
                Err(e) => eprintln!("❌ Error adding nginx config for '{}': {}", host.domain, e),
            }
        } else if let Some(redirect) = &message.redirect {
            println!("➡️ Configuring redirect: {} -> {} (SSL: {})", redirect.from, redirect.to, message.ssl);
            let use_ssl = message.ssl && NGINX_MANAGER.check_ssl_certificates_exist(&redirect.from, message.ssl_staging);
            
            if use_ssl {
                println!("🔒 SSL certificate found for '{}'", redirect.from);
            } else {
                println!("🔒 SSL certificate not found for '{}'", redirect.from);
            }
            
            match NGINX_MANAGER.add_redirect_rule(&redirect.from, &redirect.to, use_ssl, message.ssl_staging) {
                Ok(_) => {
                    println!("📝 Created Nginx redirect config from '{}' to '{}'", redirect.from, redirect.to);
                    
                    match NGINX_MANAGER.validate_nginx_config() {
                        Ok(_) => {
                            println!("✅ Nginx configuration is valid");
                            match NGINX_MANAGER.reload_nginx() {
                                Ok(_) => {
                                    println!("✅ Nginx configuration reloaded successfully");
                                    if !message.skip_certification {
                                        CertEnqueue::message(redirect.from.clone(), message.clone());
                                        print!("🔒 Certificate request queued for '{}'", redirect.from);
                                    }
                                    else {
                                        println!("🔒 Skipping certificate request for '{}'", redirect.from);
                                    }
                                },
                                Err(e) => eprintln!("❌ Failed to reload nginx: {}", e),
                            }
                        },
                        Err(e) => {
                            eprintln!("❌ Nginx configuration validation failed:\n{}", e);
                            match NGINX_MANAGER.remove_rule(&redirect.from) {
                                Ok(_) => println!("✅ Invalid configuration removed successfully"),
                                Err(e) => eprintln!("❌ Failed to remove invalid configuration: {}", e),
                            }
                        }
                    }
                }
                Err(e) => eprintln!("❌ Error adding nginx redirect config for '{}': {}", redirect.from, e),
            }
        }
    }

    pub async fn process_die_action(message: &NginxQueueMessage) {
        if let Some(host) = &message.host {
            // Clear pending messages from both queues
            NginxDequeue::clear_pending_messages(&host.domain);
            CertDequeue::clear_pending_messages(&host.domain);
    
            println!("🏠 Removing host: {}", host.domain);
            match NGINX_MANAGER.remove_rule(&host.domain) {
                Ok(_) => {
                    println!("✅ Removed Nginx config for '{}'", host.domain);
                    match NGINX_MANAGER.reload_nginx() {
                        Ok(_) => println!("✅ Nginx configuration reloaded successfully"),
                        Err(e) => eprintln!("❌ Failed to reload nginx: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Error removing nginx config for '{}': {}", host.domain, e),
            }
        } else if let Some(redirect) = &message.redirect {
            // Clear pending messages from both queues
            NginxDequeue::clear_pending_messages(&redirect.from);
            CertDequeue::clear_pending_messages(&redirect.from);
    
            println!("➡️ Removing redirect: {}", redirect.from);
            match NGINX_MANAGER.remove_rule(&redirect.from) {
                Ok(_) => {
                    println!("✅ Removed Nginx redirect config for '{}'", redirect.from);
                    match NGINX_MANAGER.reload_nginx() {
                        Ok(_) => println!("✅ Nginx configuration reloaded successfully"),
                        Err(e) => eprintln!("❌ Failed to reload nginx: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Error removing nginx redirect config for '{}': {}", redirect.from, e),
            }
        }
    }

    pub async fn start() {
        loop {
            if !NginxDequeue::is_empty() {
                if let Some(message) = NginxDequeue::message() {
                    // Check if the message is ready to be processed
                    let now = Utc::now();
                    if let Some(delay_until) = message.delay_until {
                        if now < delay_until {
                            NginxEnqueue::message_with_delay(message.clone(), std::time::Duration::from_secs(message.delay_seconds.unwrap_or(0)), message.skip_certification);
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                    }
                    
                    // Process the message as normal
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
            else {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        }
    }
}