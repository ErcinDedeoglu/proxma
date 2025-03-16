use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webserver {
    /// Sets the maximum allowed size of the client request body
    #[serde(default = "default_client_max_body_size")]
    pub client_max_body_size: String, // m for megabytes, k for kilobytes, g for gigabytes, usage: 1m, 1k, 1g

    /// Defines a timeout for reading the client request body
    #[serde(default = "default_client_body_timeout")]
    pub client_body_timeout: String,

    /// Defines a timeout for reading the client request header (= client_body_timeout for uniformity)
    #[serde(default = "default_client_header_timeout")]
    pub client_header_timeout: String,

    /// Sets a timeout for transmitting a response to the client
    #[serde(default = "default_send_timeout")]
    pub send_timeout: String,

    /// Sets the maximum time to keep a connection open with the client
    #[serde(default = "default_keepalive_timeout")]
    pub keepalive_timeout: String,

    /// Sets the maximum time to establish a connection with the upstream server
    #[serde(default = "default_proxy_connect_timeout")]
    pub proxy_connect_timeout: String,

    /// Sets the maximum time to send a request to the upstream server
    #[serde(default = "default_proxy_send_timeout")]
    pub proxy_send_timeout: String,

    /// Sets the maximum time to wait for a response from the upstream server
    #[serde(default = "default_proxy_read_timeout")]
    pub proxy_read_timeout: String,
}

fn default_client_max_body_size() -> String {
    "100m".to_string() // 100 MB
}

fn default_client_body_timeout() -> String {
    "100s".to_string() // 100 seconds
}

fn default_client_header_timeout() -> String {
    "100s".to_string() // 100 seconds
}

fn default_send_timeout() -> String {
    "30s".to_string() // 30 seconds
}

fn default_keepalive_timeout() -> String {
    "400s".to_string() // 400 seconds
}

fn default_proxy_connect_timeout() -> String {
    "15s".to_string() // 15 seconds
}

fn default_proxy_send_timeout() -> String {
    "30s".to_string() // 30 seconds
}

fn default_proxy_read_timeout() -> String {
    "100s".to_string() // 100 seconds
}

impl Default for Webserver {
    fn default() -> Self {
        Self {
            client_max_body_size: default_client_max_body_size(),
            client_body_timeout: default_client_body_timeout(),
            client_header_timeout: default_client_header_timeout(),
            send_timeout: default_send_timeout(),
            keepalive_timeout: default_keepalive_timeout(),
            proxy_connect_timeout: default_proxy_connect_timeout(),
            proxy_send_timeout: default_proxy_send_timeout(),
            proxy_read_timeout: default_proxy_read_timeout(),
        }
    }
}