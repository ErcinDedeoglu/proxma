use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};
use crate::nginx::nginx_templates::generate_proxy_server_block;

pub struct NginxManager {
    state: Mutex<NginxState>,
    webroot_path: PathBuf,
}

struct NginxState {
    config_path: PathBuf,
}

impl NginxManager {
    pub fn new<P: AsRef<Path>, W: AsRef<Path>>(config_path: P, webroot_path: W) -> Self {
        Self {
            state: Mutex::new(NginxState {
                config_path: config_path.as_ref().to_path_buf(),
            }),
            webroot_path: webroot_path.as_ref().to_path_buf(),
        }
    }

    /// Directly receives a fully built "upstream_url"
    pub fn add_container_host_rule(
        &self,
        domain: &str,
        upstream_url: &str,
        ssl: bool,
    ) -> io::Result<()> {
        let webroot = self.webroot_path.to_str().unwrap_or("/var/www/html");

        let config_content = generate_proxy_server_block(domain, upstream_url, ssl, webroot);

        let file_name = format!("proxma_{}.conf", domain.replace('.', "_"));
        let file_path = self.state.lock().unwrap().config_path.join(file_name);

        fs::write(&file_path, config_content)?;

        println!(
            "📝 Created Nginx config for '{}', proxy to '{}'",
            domain, upstream_url
        );

        Ok(())
    }

    pub fn reload_nginx(&self) -> io::Result<()> {
        std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .output()?;

        println!("🔄 Nginx configuration reloaded successfully.");
        Ok(())
    }
}