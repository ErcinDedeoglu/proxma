mod docker;
mod nginx;
mod certbot;
mod queue;

use futures::StreamExt;
use queue::Enqueue;
use tokio::time::sleep;
use std::time::Duration;
use queue::Dequeue;

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        process_queue().await;
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

async fn process_queue() {
    loop {
        if !Dequeue::is_empty() {
            if let Some(message) = Dequeue::message() {
                match message.action.as_str() {
                    "start" => {
                        if message.hosting {
                            if let Some(host) = message.host {
                                println!("Processing host: {} on port {}", host.domain, host.port);
                                // Process host configuration
                                // For example: nginx::configure_host(&host, message.ssl);
                            }
                        } else if let Some(redirect) = message.redirect {
                            println!("Processing redirect: {} -> {}", redirect.from, redirect.to);
                            // Process redirect configuration
                            // For example: nginx::configure_redirect(&redirect);
                        }
                    },
                    "die" => {
                        if message.hosting {
                            if let Some(host) = message.host {
                                println!("Removing host: {}", host.domain);
                                // Remove host configuration
                                // For example: nginx::remove_host(&host);
                            }
                        } else if let Some(redirect) = message.redirect {
                            println!("Removing redirect: {}", redirect.from);
                            // Remove redirect configuration
                            // For example: nginx::remove_redirect(&redirect);
                        }
                    },
                    _ => {}
                }
            }
        }
        sleep(Duration::from_millis(1000)).await;
    }
}