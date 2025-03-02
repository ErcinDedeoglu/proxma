mod docker;

use futures::StreamExt;

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
    }
}