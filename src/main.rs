mod docker;
mod nginx;
mod certbot;

use futures::StreamExt;
use crate::nginx::{NginxManager, ProxyRule};
use crate::certbot::Certbot;

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
                    
                    // Create and add proxy rule
                    let rule = ProxyRule {
                        id: event.name.clone(),
                        domains: domains.clone(),
                        upstream: format!("http://{}:{}", event.name, port),
                        redirects,
                        ssl,
                    };
                    
                    match nginx_manager.add_rule(rule) {
                        Ok(_) => {
                            println!("✅ Added proxy rule for container: {}", event.name);
                            
                            // Request SSL certificates if needed
                            if ssl {
                                let domain_refs: Vec<&str> = domains.iter().map(|s| s.as_str()).collect();
                                match certbot.request_certificate(&domain_refs) {
                                    Ok(_) => println!("🔒 Requested SSL certificates for: {}", domains.join(", ")),
                                    Err(e) => eprintln!("⚠️ Failed to request SSL certificates: {}", e),
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
                    match nginx_manager.remove_rule_by_id(&event.name) {
                        Ok(_) => println!("🗑️ Removed proxy rule for container: {}", event.name),
                        Err(e) => eprintln!("❌ Failed to remove Nginx proxy rule: {}", e),
                    }
                }
            },
            _ => {}, // Ignore other events
        }
    }
    
    println!("⚠️ Event stream ended unexpectedly");
}

fn setup_nginx_example() -> Result<(), Box<dyn std::error::Error>> {
    let manager = NginxManager::new("/etc/nginx/conf.d/proxma-proxy-rules.conf", "/var/www/html");

    if let Err(e) = manager.add_rule(ProxyRule {
        id: "ercin.info".into(),
        domains: vec![
            "ercin.info".into()
        ],
        upstream: "http://ercin.info:80".into(),
        redirects: vec![
            ("x1.ercin.info".into(), "ercin.info".into()),
            ("www.ercin.info".into(), "ercin.info".into()),
            ("blog1.ercin.info".into(), "ercin.info".into()),
            ("blog2.ercin.info".into(), "ercin.info".into()),
            ("blog3.ercin.info".into(), "ercin.info".into()),
            ("blog4.ercin.info".into(), "ercin.info".into()),
            ("blog5.ercin.info".into(), "ercin.info".into()),
        ],
        ssl: true
    }) {
        eprintln!("Failed to add Nginx proxy rule: {}", e);
        // Continue execution despite the error
    } else {
        println!("Successfully added Nginx rule for ercin.info");
    }

    // // Remove the rule by its ID
    // manager.remove_rule_by_id("ercin.info")?;

    // // Alternatively, remove a rule by one of its domains
    // manager.remove_rule_by_domain("api.myapp.com")?;

    Ok(())
}

fn setup_certbot_example() -> Result<(), Box<dyn std::error::Error>> {
    let certbot = Certbot::new("/var/www/html", "dublokcom@gmail.com")
        .agree_tos(true)
        .staging(true)
        .no_eff_email(false)
        .config_dir("/var/proxma/letsencrypt")
        .work_dir("/var/proxma/letsencrypt/work")
        .logs_dir("/var/proxma/logs");
    
    // Handle error specifically for certificate requests
    if let Err(e) = certbot.request_certificate(&["ercin.info", "www.ercin.info"]) {
        eprintln!("Certificate provisioning failed: {}", e);
        // Continue execution despite the error
    } else {
        println!("Successfully requested certificates for domains");
    }
  
    Ok(())
}
