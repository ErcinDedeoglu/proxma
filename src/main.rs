mod docker;
mod nginx;
mod dns;
mod certbot;
mod queue;
mod models;
mod domain_tracker;
pub mod acme_helper;

use futures::StreamExt;
use queue::{NginxEnqueue, NginxQueueProcessor, CertQueueProcessor};
use domain_tracker::DomainTracker;

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        NginxQueueProcessor::start().await;
    });
    
    tokio::spawn(async {
        CertQueueProcessor::start().await;
    });
    
    // Spawn orphan cleanup task that waits for initial processing to complete
    tokio::spawn(async {
        // Wait for initial container discovery and processing
        let mut initial_processing_done = false;
        let mut stable_count = 0;
        
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let queue_size = NginxEnqueue::size();
            
            if queue_size == 0 {
                stable_count += 1;
                // Wait for queue to be stable (empty) for 3 consecutive checks
                if stable_count >= 3 && !initial_processing_done {
                    println!("🔍 Initial processing complete, performing orphan cleanup...");
                    match DomainTracker::cleanup_orphaned_configs() {
                        Ok(removed_domains) => {
                            if removed_domains.is_empty() {
                                println!("✅ No orphaned configurations found");
                            } else {
                                println!("🧹 Cleaned up {} orphaned configurations", removed_domains.len());
                            }
                        },
                        Err(e) => {
                            eprintln!("❌ Failed to cleanup orphaned configs: {}", e);
                        }
                    }
                    initial_processing_done = true;
                    break;
                }
            } else {
                stable_count = 0; // Reset if queue is not empty
            }
        }
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