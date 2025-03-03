mod docker;
mod nginx;
mod certbot;

use futures::StreamExt;
use crate::nginx::{NginxManager, ProxyRule};
use crate::certbot::Certbot;

#[tokio::main]
async fn main() {
    setup_nginx_example().expect("Nginx setup failed");
    setup_certbot_example().expect("Certbot certificate provisioning failed");

    let mut stream = docker::stream_container_events()
        .await
        .expect("Event stream failure");
    
    println!("🚀 Container events:");
    while let Some(event) = stream.next().await {
        println!(
            "[{}] ID: {}, Image: {}, Name: {}, Labels: {:?}, Networks: {:?}",
            event.action, event.container_id, event.image, event.name, event.labels, event.networks
        );
    }
}

fn setup_nginx_example() -> Result<(), Box<dyn std::error::Error>> {
    let manager = NginxManager::new("/etc/nginx/conf.d/proxma-proxy-rules.conf", "/var/www/html");

    if let Err(e) = manager.add_rule(ProxyRule {
        id: "ercin.info".into(),
        domains: vec![
            "ercin.info".into(),
            "www.ercin.info".into()
        ],
        upstream: "http://ercin.info:80".into(),
        redirects: vec![
            ("www.ercin.info".into(), "ercin.info".into()),
            ("blog.ercin.info".into(), "ercin.info".into()),
        ],
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
        .no_eff_email(false);  
    
    // Handle error specifically for certificate requests
    if let Err(e) = certbot.request_certificate(&["ercin.info", "www.ercin.info"]) {
        eprintln!("Certificate provisioning failed: {}", e);
        // Continue execution despite the error
    } else {
        println!("Successfully requested certificates for domains");
    }
  
    Ok(())
}
