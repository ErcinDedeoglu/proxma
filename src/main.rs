mod docker;
mod nginx;
mod certbot;
mod queue;

use futures::StreamExt;
use queue::Enqueue;

#[tokio::main]
async fn main() {    
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
                if Enqueue::should_process_container(&event.labels) {
                    Enqueue::process_container_event(
                        event.action,
                        event.container_id,
                        event.name,
                        event.image,
                        event.networks,
                        event.labels,
                    );
                    println!("Queue size: {}", Enqueue::size());
                }
            },
            _ => {},
        }
    }
    
    println!("⚠️ Event stream ended unexpectedly");
}