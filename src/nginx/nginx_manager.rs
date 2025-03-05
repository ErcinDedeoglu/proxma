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
        // Generate nginx configuration content
        let config_content = generate_proxy_server_block(
            domain,
            upstream_url,
            ssl,
            self.webroot_path.to_str().unwrap_or_default(),
        );

        // Create file name and path
        let file_name = format!("proxma_{}.conf", domain.replace('.', "_"));
        let file_path = self.state.lock().unwrap().config_path.join(&file_name);

        // Write the new nginx configuration file
        fs::write(&file_path, config_content)?;

        println!(
            "📝 Created Nginx config for '{}', proxy to '{}'",
            domain, upstream_url
        );

        // Validate the newly written nginx configuration
        match self.validate_nginx_config() {
            Ok(()) => {
                // Reload nginx to apply the new configuration
                self.reload_nginx()?;
                println!("🚀 Nginx reloaded successfully with the new configuration.");
                Ok(())
            }
            Err(e) => {
                // Remove the invalid config file to restore previous valid state
                eprintln!(
                    "⚠ Configuration validation failed, removing problematic configuration '{}'",
                    file_path.display()
                );
                fs::remove_file(&file_path)?;
                Err(e)
            }
        }
    }

    /// Validates the current Nginx configuration.
    pub fn validate_nginx_config(&self) -> io::Result<()> {
        let output = std::process::Command::new("nginx")
            .arg("-t")
            .output()?;

        // Check if nginx validation command succeeded
        if output.status.success() {
            println!("✅ Nginx configuration is valid.");
            Ok(())
        } else {
            // If validation fails, print out the stderr from the nginx command.
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("⚠️ Nginx configuration validation failed:\n{}", stderr);
            Err(io::Error::new(io::ErrorKind::Other, "Nginx validation failed"))
        }
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