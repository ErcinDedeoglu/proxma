mod docker;
mod nginx;
mod certbot;

use futures::StreamExt;
use crate::nginx::{NginxManager, ProxyRule};
use crate::certbot::Certbot;

#[tokio::main]
async fn main() {
    setup_nginx_example().expect("Nginx setup failed");
    // setup_certbot_example().expect("Certbot certificate provisioning failed");

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
    let mut manager = NginxManager::new("/etc/nginx/conf.d/proxma-proxy-rules.conf", "/var/www/html");

    manager.add_rule(ProxyRule {
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
    })?;

    // // Remove the rule by its ID
    // manager.remove_rule_by_id(rule_id)?;

    // // Alternatively, remove a rule by one of its domains
    // manager.remove_rule_by_domain("api.myapp.com")?;

    Ok(())
}

fn setup_certbot_example() -> Result<(), Box<dyn std::error::Error>> {
    let certbot = Certbot::new("/var/www/html", "dublokcom@gmail.com")
        .agree_tos(true)
        .staging(true)
        .no_eff_email(false);  
    
    certbot.request_certificate(&["ercin.info", "www.ercin.info"])?;
  
    Ok(())
}
