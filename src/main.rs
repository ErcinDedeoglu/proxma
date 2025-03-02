mod docker;
mod nginx;

use futures::StreamExt;
use crate::nginx::{NginxManager, ProxyRule};

#[tokio::main]
async fn main() {
    // Example nginx setup (in separate method)
    setup_nginx_example().expect("Nginx setup failed");

    // Existing event loop remains unchanged
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
    let mut manager = NginxManager::new("/etc/nginx/conf.d/proxma-proxy-rules.conf");

    manager.add_rule(ProxyRule {
        id: "ercin.info".into(),
        domains: vec![
            "ercin.info".into(),
            "www.ercin.info".into()
        ],
        upstream: "http://ercin.info:80".into(),
    })?;

    // // Remove the rule by its ID
    // manager.remove_rule_by_id(rule_id)?;

    // // Alternatively, remove a rule by one of its domains
    // manager.remove_rule_by_domain("api.myapp.com")?;

    Ok(())
}