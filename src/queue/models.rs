#[derive(Debug)]
pub struct QueueMessage {
    pub action: String,
    pub container_id: String,
    pub name: String,
    pub image: String,
    pub networks: Vec<String>,
    pub hosting: bool,
    pub ssl: bool,
    pub host: Option<Host>,
    pub redirect: Option<Redirect>,
}

#[derive(Debug)]
pub struct Host {
    pub domain: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct Redirect {
    pub from: String,
    pub to: String,
}