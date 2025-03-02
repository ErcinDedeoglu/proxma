use bollard::{
    Docker, errors::Error, container::{ListContainersOptions, InspectContainerOptions},
    models::{EventMessage, ContainerSummary}, system::EventsOptions,
};
use futures::{Stream, StreamExt};
use std::{sync::Arc, collections::HashMap, pin::Pin};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ContainerEvent {
    pub container_id: String,
    pub image: String,
    pub name: String,
    pub action: String,
    pub labels: HashMap<String, String>,
    pub networks: Vec<String>,
}

async fn inspect_networks(docker: &Arc<Docker>, id: &str) -> Vec<String> {
    docker.inspect_container(id, None::<InspectContainerOptions>).await.ok()
        .and_then(|d| d.network_settings?.networks)
        .map(|n| n.into_keys().collect()).unwrap_or_default()
}

async fn summary_to_event(docker: Arc<Docker>, summary: &ContainerSummary, action: &str) -> Option<ContainerEvent> {
    let container_id = summary.id.as_ref()?;
    Some(ContainerEvent {
        container_id: container_id.clone(),
        image: summary.image.clone()?,
        name: summary.names.as_ref()?.iter().next()?.trim_start_matches('/').into(),
        action: action.into(),
        labels: summary.labels.clone().unwrap_or_default(),
        networks: inspect_networks(&docker, container_id).await,
    })
}

async fn event_message_to_event(docker: Arc<Docker>, event: &EventMessage) -> Option<ContainerEvent> {
    let actor = event.actor.as_ref()?;
    let container_id = actor.id.as_ref()?;
    let attributes = actor.attributes.clone().unwrap_or_default();
    Some(ContainerEvent {
        container_id: container_id.clone(),
        image: attributes.get("image").cloned().unwrap_or_default(),
        name: attributes.get("name").cloned().unwrap_or_default(),
        action: event.action.as_ref()?.into(),
        labels: attributes,
        networks: inspect_networks(&docker, container_id).await,
    })
}

pub async fn stream_container_events() -> Result<Pin<Box<dyn Stream<Item=ContainerEvent> + Send>>> {
    let docker = Arc::new(Docker::connect_with_local_defaults()?);

    let initial = docker.list_containers(Some(ListContainersOptions::<String> {
        all: false, ..Default::default()
    })).await?;
    
    let docker_initial = docker.clone();
    let initial_stream = futures::stream::iter(initial).then(move |summary| {
        let docker = docker_initial.clone();
        async move { summary_to_event(docker, &summary, "start").await }
    }).filter_map(|e| async move { e });

    // create → start → (running) → die → stop → destroy    
    let mut filters = HashMap::new();
    filters.insert("type".to_string(), vec!["container".to_string()]);
    filters.insert(
        "event".to_string(),
        vec![
            // "create".to_string(),
            "start".to_string(),
            // "stop".to_string(),
            // "restart".to_string(),
            "die".to_string(),
            // "destroy".to_string(),
            // "pause".to_string(),
            // "unpause".to_string(),
            // Add more events if needed, but omit exec_* explicitly
        ],
    );

    let docker_live = docker.clone();
    let live_stream = docker.events(Some(EventsOptions {
        filters,
        ..Default::default()
    })).filter_map(move |e| {
        let docker = docker_live.clone();
        async move {
            match e {
                Ok(ev) => event_message_to_event(docker.clone(), &ev).await,
                Err(err) => {
                    eprintln!("Error receiving docker event: {}", err);
                    None
                }
            }
        }
    });

    Ok(Box::pin(initial_stream.chain(live_stream)))
}