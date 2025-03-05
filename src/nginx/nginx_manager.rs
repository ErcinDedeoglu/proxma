use std::path::{Path, PathBuf};
use std::sync::Mutex;
use crate::nginx::nginx_proxy_rule::ProxyRule;
use crate::nginx::nginx_templates::{generate_proxy_server_block, generate_redirect_server_block};
use std::fs;
use std::io::{self, Error, ErrorKind};
use super::models::{Host, Redirect};
use super::nginx_templates::{generate_proxy_server_block, generate_redirect_server_block};

pub struct NginxManager {
    // Single mutex for all state and operations
    state: Mutex<NginxState>,
    webroot_path: PathBuf,
}

struct NginxState {
    rules: Vec<ProxyRule>,
    config_path: PathBuf,
}

impl NginxManager {
    pub fn new<P: AsRef<Path>, W: AsRef<Path>>(config_path: P, webroot_path: W) -> Self {
        Self {
            state: Mutex::new(NginxState {
                rules: Vec::new(),
                config_path: config_path.as_ref().to_path_buf(),
            }),
            webroot_path: webroot_path.as_ref().to_path_buf(),
        }
    }

    pub fn add_rule(&self, host: &Host, ssl: bool) -> io::Result<()> {
        let webroot = self.webroot_path.to_str().unwrap_or("/var/www/html");
        let upstream = format!("http://localhost:{}", host.port);
        
        // Generate Nginx configuration
        let config_content = generate_proxy_server_block(&host.domain, &upstream, ssl, webroot);
        
        // Create config file
        let file_name = format!("proxma_{}.conf", host.domain.replace(".", "_"));
        let file_path = self.state.lock().unwrap().config_path.join(file_name);
        
        // Write configuration to file
        fs::write(&file_path, config_content)?;
        
        println!("Created Nginx config for host: {}", host.domain);
        Ok(())
    }
    
    pub fn reload_nginx(&self) -> io::Result<()> {
        std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .output()?;
        
        println!("Nginx configuration reloaded");
        Ok(())
    }
}