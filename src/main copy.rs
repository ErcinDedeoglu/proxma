mod docker;
mod nginx;
mod certbot;

use futures::StreamExt;
use crate::nginx::{NginxManager, ProxyRule};
use crate::certbot::Certbot;
use crate::certbot::CertificateRequestResult;

#[tokio::main]
async fn main() {
    // Initialize NginxManager and Certbot
    let nginx_manager = NginxManager::new("/etc/nginx/conf.d/proxma-proxy-rules.conf", "/var/www/html");
    let certbot = Certbot::new("/var/www/html", "dublokcom@gmail.com")
        .agree_tos(true)
        .staging(true)
        .no_eff_email(false)
        .config_dir("/var/proxma/letsencrypt")
        .work_dir("/var/proxma/letsencrypt/work")
        .logs_dir("/var/proxma/logs");
    
    // Track active rules by container name for better cleanup
    let mut active_containers: std::collections::HashMap<String, (Vec<String>, Vec<String>)> = std::collections::HashMap::new();
    
    // Start streaming container events
    let mut stream = docker::stream_container_events()
        .await
        .expect("Event stream failure");
    
    println!("🚀 Container events:");
    while let Some(event) = stream.next().await {
        println!(
            "[{}] ID: {}, Image: {}, Name: {}, Labels: {:?}, Networks: {:?}",
            event.action, event.container_id, event.image, event.name, event.labels, event.networks
        );
        
        match event.action.as_str() {
            "start" => {
                // Process containers with proxma.hosts label
                if let Some(hosts_str) = event.labels.get("proxma.hosts") {
                    let domains: Vec<String> = hosts_str.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    if domains.is_empty() {
                        continue;
                    }
                    
                    // Get port (default to 80)
                    let port = event.labels.get("proxma.port")
                        .map(|p| p.trim())
                        .unwrap_or("80");
                    
                    // Check if SSL is enabled
                    let ssl = event.labels.get("proxma.ssl")
                        .map(|v| v.trim().to_lowercase() == "true")
                        .unwrap_or(false);
                    
                    // Parse redirects
                    let redirects = event.labels.get("proxma.redirects")
                        .map(|redirect_str| {
                            redirect_str.split(',')
                                .filter_map(|r| {
                                    let parts: Vec<&str> = r.trim().split(':').collect();
                                    if parts.len() == 2 {
                                        Some((parts[0].to_string(), parts[1].to_string()))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_else(Vec::new);
                    
                    // Track redirect domains for cleanup
                    let redirect_domains: Vec<String> = redirects
                        .iter()
                        .map(|(from, _)| from.clone())
                        .collect();
                    
                    // Store domains for later reference during die events
                    active_containers.insert(
                        event.name.clone(),
                        (domains.clone(), redirect_domains.clone())
                    );
                    
                    // Create and add proxy rule
                    let rule = ProxyRule {
                        id: event.name.clone(),
                        domains: domains.clone(),
                        upstream: format!("http://{}:{}", event.name, port),
                        redirects: redirects.clone(),
                        ssl,
                    };
                    
                    let rule_clone = rule.clone();

                    match nginx_manager.add_rule(rule) {
                        Ok(_) => {
                            println!("✅ Added proxy rule for container: {}", event.name);
                            println!("   - Main domains: {}", domains.join(", "));
                            if !redirect_domains.is_empty() {
                                println!("   - Redirect domains: {}", redirect_domains.join(", "));
                            }
                            
                            // Request SSL certificates if needed
                            if ssl {
                                // Collect all domains needing certificates (main domains + redirect sources)
                                let mut all_domains = domains.clone();
                                all_domains.extend(redirect_domains.clone());
                                all_domains.sort();
                                all_domains.dedup();
                            
                                for domain in &all_domains {
                                    match certbot.check_and_request_certificate(domain) {
                                        Ok(result) => match result {
                                            CertificateRequestResult::Success => {
                                                println!("🔒 Requested SSL certificate successfully for domain: {}", domain);
                                                let ssl_certificate_exist: bool = nginx_manager.check_ssl_certificates_exist(domain);
                                                if ssl_certificate_exist {
                                                    println!("🔒✅ SSL certificate exists for domain: {}", domain);
                                                    match nginx_manager.add_rule(rule_clone.clone()) {
                                                        Ok(_) => println!("✅ Updated proxy rule with SSL for domain: {}", domain),
                                                        Err(e) => eprintln!("❌ Failed to update Nginx proxy rule with SSL: {}", e),
                                                    }
                                                } else {
                                                    println!("🔒❌ SSL certificate does not exist for domain: {}", domain);
                                                }
                                            },
                                            CertificateRequestResult::AcmeChallengeFailure(error) => {
                                                eprintln!("⚠️ ACME challenge failed for domain '{}': {}", domain, error);
                                                eprintln!("⚠️ Please ensure the domain is properly configured and points to this server.");
                                            },
                                            CertificateRequestResult::CertbotError(error) => {
                                                eprintln!("⚠️ Certbot command failed for domain '{}': {}", domain, error);
                                            }
                                        },
                                        Err(e) => eprintln!("⚠️ System error checking domain '{}': {}", domain, e),
                                    }
                                }
                            }
                        },
                        Err(e) => eprintln!("❌ Failed to add Nginx proxy rule: {}", e),
                    }
                }
            },
            "die" => {
                // Remove proxy rule when container dies
                if event.labels.contains_key("proxma.hosts") {
                    // Get domains that were associated with this container
                    if let Some((main_domains, redirect_domains)) = active_containers.remove(&event.name) {
                        // Track if we've successfully removed at least one rule
                        let mut success = false;
                        
                        // Remove rules for each main domain
                        for domain in &main_domains {
                            match nginx_manager.remove_rule_by_domain(domain) {
                                Ok(_) => {
                                    success = true;
                                    println!("   - Removed domain: {}", domain);
                                },
                                Err(e) => eprintln!("❌ Failed to remove Nginx proxy rule for domain '{}': {}", domain, e),
                            }
                        }
                        
                        // Remove rules for each redirect domain
                        for domain in &redirect_domains {
                            match nginx_manager.remove_rule_by_domain(domain) {
                                Ok(_) => {
                                    success = true;
                                    println!("   - Removed redirect domain: {}", domain);
                                },
                                Err(e) => eprintln!("❌ Failed to remove Nginx proxy rule for redirect domain '{}': {}", domain, e),
                            }
                        }
                        
                        if success {
                            println!("🗑️ Removed proxy rules for container: {}", event.name);
                        }
                    } else {
                        eprintln!("⚠️ Container had proxma.hosts label but no domains were tracked: {}", event.name);
                    }
                }
            }
            _ => {},
        }
    }
    
    println!("⚠️ Event stream ended unexpectedly");
}