mod docker;
mod nginx;
mod certbot;
mod queue;
pub mod acme_helper;

use futures::StreamExt;
use queue::{NginxEnqueue, NginxQueueProcessor, CertQueueProcessor};

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        NginxQueueProcessor::start().await;
    });
    
    tokio::spawn(async {
        CertQueueProcessor::start().await;
    });
    

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
            "start" | "die" => {
                if NginxEnqueue::should_process_container(&event.labels) {
                    NginxEnqueue::process_container_event(
                        event.action,
                        event.container_id,
                        event.name,
                        event.image,
                        event.networks,
                        event.labels,
                    );
                    println!("Queue size: {}", NginxEnqueue::size());
                }
            },
            _ => {},
        }
    }
    
    println!("⚠️ Event stream ended unexpectedly");
}