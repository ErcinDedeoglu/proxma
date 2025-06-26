use super::NginxDequeue;
use super::models::NginxQueueMessage;
use crate::nginx::NginxManager;
use crate::dns::DNSManager;
use crate::queue::cert_enqueue::CertEnqueue;
use crate::queue::{CertDequeue, NginxEnqueue};
use crate::domain_tracker::DomainTracker;
use chrono::Utc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref NGINX_MANAGER: NginxManager = NginxManager::new("/var/proxma/nginx/conf.d", "/var/www/html");
    pub static ref dns_manager: DNSManager = DNSManager::new();
}

pub struct NginxQueueProcessor;

impl NginxQueueProcessor {
    pub async fn process_start_action(message: &NginxQueueMessage) {        
        Self::process_dns_record(&message).await;

        if let Some(host) = &message.host {
            println!("🏠 Configuring host: {} (SSL: {})", host.domain, message.ssl);
            
            // Construct upstream URL based on protocol
            let upstream_url = match host.protocol.as_str() {
                "grpc" => format!("{}:{}", message.name, host.port),
                _ => format!("http://{}:{}", message.name, host.port),
            };
            
            // Construct display URL with protocol prefix for logging
            let display_url = match host.protocol.as_str() {
                "grpc" => format!("grpc://{}:{}", message.name, host.port),
                "https" => format!("https://{}:{}", message.name, host.port),
                _ => format!("http://{}:{}", message.name, host.port),
            };
            
            let use_ssl = message.ssl && NGINX_MANAGER.check_ssl_certificates_exist(&host.domain, message.ssl_staging);
            
            if use_ssl {
                println!("🔒 SSL certificate found for '{}'", host.domain);
            } else {
                if message.ssl {
                    println!("🔒 SSL certificate not found for '{}'", host.domain);
                } else {
                    println!("🔒 SSL disabled for '{}'", host.domain);
                }
            }
            
            match NGINX_MANAGER.add_container_host_rule(&host.domain, &upstream_url, use_ssl, message.ssl_staging, message.auth.clone(), message.webserver.clone(), &host.protocol) {
                Ok(_) => {
                    println!("📝 Created Nginx config for '{}', proxy to '{}'", host.domain, display_url);
                    
                    match NGINX_MANAGER.validate_nginx_config() {
                        Ok(_) => {
                            println!("✅ Nginx configuration is valid");
                            match NGINX_MANAGER.reload_nginx() {
                                Ok(_) => {
                                    println!("✅ Nginx configuration reloaded successfully");
                                    
                                    // Add domain to active list
                                    DomainTracker::add_domain(host.domain.clone());
                                    
                                    if message.ssl {
                                        if !message.skip_certification {
                                            CertEnqueue::message(host.domain.clone(), message.clone());
                                            println!("🔒 Certificate request queued for '{}'", host.domain);
                                        } else {
                                            println!("🔒 Skipping certificate request for '{}'", host.domain);
                                        }
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
                if message.ssl {
                    println!("🔒 SSL certificate not found for '{}'", redirect.from);
                } else {
                    println!("🔒 SSL disabled for '{}'", redirect.from);
                }
            }
                        
            match NGINX_MANAGER.add_redirect_rule(&redirect.from, &redirect.to, use_ssl, message.ssl_staging, message.webserver.clone()) {
                Ok(_) => {
                    println!("📝 Created Nginx redirect config from '{}' to '{}'", redirect.from, redirect.to);
                    
                    match NGINX_MANAGER.validate_nginx_config() {
                        Ok(_) => {
                            println!("✅ Nginx configuration is valid");
                            match NGINX_MANAGER.reload_nginx() {
                                Ok(_) => {
                                    println!("✅ Nginx configuration reloaded successfully");
                                    
                                    // Add domain to active list
                                    DomainTracker::add_domain(redirect.from.clone());
                                    
                                    if message.ssl {
                                        if !message.skip_certification {
                                            CertEnqueue::message(redirect.from.clone(), message.clone());
                                            println!("🔒 Certificate request queued for '{}'", redirect.from);
                                        } else {
                                            println!("🔒 Skipping certificate request for '{}'", redirect.from);
                                        }
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

    pub async fn process_dns_record(message: &NginxQueueMessage) {
        if message.skip_dns {
            println!("🌐 Skipping DNS record creation because skip_dns flag is set");
            return;
        } else if message.dns_provider.as_ref() != Some(&"cloudflare".to_string()) {
            println!("🌐 Skipping DNS record creation because DNS Provider is not cloudflare (provider: {:?})", message.dns_provider);
            return;
        }

        // Debug: Print what credentials we received
        println!("🔍 DNS Credentials Debug:");
        println!("   - cloudflare_email: {:?}", message.cloudflare_email.as_ref().map(|e| if e.is_empty() { "EMPTY" } else { "PROVIDED" }));
        println!("   - cloudflare_api_key: {:?}", message.cloudflare_api_key.as_ref().map(|k| if k.is_empty() { "EMPTY" } else { "PROVIDED" }));
        println!("   - cloudflare_api_token: {:?}", message.cloudflare_api_token.as_ref().map(|t| if t.is_empty() { "EMPTY" } else { "PROVIDED" }));

        let dns_record: String;

        if let Some(host) = &message.host {
            dns_record = host.domain.clone();
        }
        else if let Some(redirect) = &message.redirect {
            dns_record = redirect.from.clone();
        }
        else {
            return;
        }
        
        let dns_provider = message.dns_provider.clone().unwrap_or_default();
        let record_type = message.dns_record_type.clone().unwrap_or_default();
        let record_target = message.dns_record_target.clone().unwrap_or_default();
        let record_proxied = message.dns_record_proxied.unwrap_or_default();
        
        // Only pass credentials if they exist and are not empty
        let cloudflare_email = message.cloudflare_email.clone().filter(|s| !s.is_empty());
        let cloudflare_api_key = message.cloudflare_api_key.clone().filter(|s| !s.is_empty());
        let cloudflare_api_token = message.cloudflare_api_token.clone().filter(|s| !s.is_empty());

        // Check if we have valid credentials before proceeding
        let has_email_key = cloudflare_email.is_some() && cloudflare_api_key.is_some();
        let has_token = cloudflare_api_token.is_some();
        
        if !has_email_key && !has_token {
            eprintln!("❌ DNS record creation failed: No valid Cloudflare credentials provided");
            eprintln!("   Please provide either:");
            eprintln!("   - cloudflare_email AND cloudflare_api_key, OR");
            eprintln!("   - cloudflare_api_token");
            return;
        }

        // Test connectivity before proceeding with DNS operations
        use crate::dns::RecordManager;
        if let Err(e) = RecordManager::test_connectivity(
            cloudflare_email.clone(),
            cloudflare_api_key.clone(),
            cloudflare_api_token.clone()
        ).await {
            eprintln!("❌ Cloudflare API connectivity test failed: {}", e);
            eprintln!("   This may indicate network connectivity issues or invalid credentials");
            // Continue with the operation anyway, but user will see the connectivity issue
        }

        println!("🌐 Creating DNS record for '{}' -> '{}' (type: {}, proxied: {})",
                 dns_record, record_target, record_type, record_proxied);

        let success = dns_manager.add_update_record(
            dns_provider,
            dns_record,
            record_target,
            record_type,
            record_proxied,
            cloudflare_email,
            cloudflare_api_key,
            cloudflare_api_token
        ).await;
        
        if success {
            println!("✅ DNS record created/updated successfully");
        } else {
            eprintln!("❌ DNS record creation/update failed");
        }
    }

    pub async fn process_die_action(message: &NginxQueueMessage) {
        if let Some(host) = &message.host {
            // Clear pending messages from both queues
            NginxDequeue::clear_pending_messages(&host.domain, &message.container_id);
            CertDequeue::clear_pending_messages(&host.domain);
    
            println!("🏠 Removing host: {}", host.domain);
            
            // Remove domain from active list
            DomainTracker::remove_domain(&host.domain);
            
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
            NginxDequeue::clear_pending_messages(&redirect.from, &message.container_id);
            CertDequeue::clear_pending_messages(&redirect.from);
    
            println!("➡️ Removing redirect: {}", redirect.from);
            
            // Remove domain from active list
            DomainTracker::remove_domain(&redirect.from);
            
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