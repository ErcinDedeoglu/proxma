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

    pub fn add_container_host_rule(
        &self,
        domain: &str,
        upstream_url: &str,
        ssl: bool,
    ) -> io::Result<()> {
        let config_content = generate_proxy_server_block(
            domain,
            upstream_url,
            ssl,
            self.webroot_path.to_str().unwrap_or_default(),
        );
    
        let file_name = format!("proxma_{}.conf", domain.replace('.', "_"));
        let file_path = self.state.lock().unwrap().config_path.join(&file_name);
    
        fs::write(&file_path, config_content)?;
        println!(
            "📝 Created Nginx config for '{}', proxy to '{}'",
            domain, upstream_url
        );
        Ok(())
    }

    pub fn validate_nginx_config(&self) -> io::Result<()> {
        let output = std::process::Command::new("nginx")
            .arg("-t")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("⚠️ Nginx configuration validation failed:\n{}", stderr);
            return Err(io::Error::new(io::ErrorKind::Other, "Nginx validation failed"));
        }

        println!("✅ Nginx configuration is valid.");
        Ok(())
    }

    pub fn remove_rule(&self, domain: &str) -> io::Result<()> {
        let file_name = format!("proxma_{}.conf", domain.replace('.', "_"));
        let file_path = self.state.lock().unwrap().config_path.join(&file_name);
        
        if let Err(e) = fs::remove_file(&file_path) {
            eprintln!("❌ Failed to remove configuration file: {}", e);
            return Err(e);
        }
        
        println!("🗑️ Removed configuration file: {}", file_path.display());
        Ok(())
    }

    pub fn reload_nginx(&self) -> io::Result<()> {
        let output = std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .output()?;

        if output.status.success() {
            println!("🔄 Nginx configuration reloaded successfully.");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("⚠️ Failed to reload nginx:\n{}", stderr);
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to reload nginx configuration",
            ))
        }
    }
}