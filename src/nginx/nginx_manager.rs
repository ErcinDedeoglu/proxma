use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};
use crate::nginx::nginx_templates::{generate_proxy_server_block, generate_grpc_server_block};
use crate::nginx::nginx_templates::generate_redirect_server_block;
use crate::models::{Auth, Webserver};

use super::nginx_templates::sanitize_domain;

pub struct NginxManager {
    state: Mutex<NginxState>,
    webroot_path: PathBuf,
}

struct NginxState {
    config_path: PathBuf,
}

impl NginxManager {
    /// Convert domain to config filename
    pub fn domain_to_filename(domain: &str) -> String {
        let sanitized_domain = sanitize_domain(domain);
        format!("proxma_{}.conf", sanitized_domain)
    }


    pub fn new<P: AsRef<Path>, W: AsRef<Path>>(config_path: P, webroot_path: W) -> Self {
        let config_path_buf = config_path.as_ref().to_path_buf();
        
        // Ensure the nginx config directory exists
        if let Err(e) = fs::create_dir_all(&config_path_buf) {
            eprintln!("⚠️ Failed to create nginx config directory {:?}: {}", config_path_buf, e);
        } else {
            println!("📁 Nginx config directory ready: {:?}", config_path_buf);
        }
        
        Self {
            state: Mutex::new(NginxState {
                config_path: config_path_buf,
            }),
            webroot_path: webroot_path.as_ref().to_path_buf(),
        }
    }

    pub fn add_redirect_rule(
        &self,
        from_domain: &str,
        to_domain: &str,
        ssl: bool,
        ssl_staging: bool,
        webserver: Webserver,
    ) -> io::Result<()> {
        let config_content = generate_redirect_server_block(
            from_domain,
            to_domain,
            ssl,
            self.webroot_path.to_str().unwrap_or_default(),
            ssl_staging,
            webserver,
        );

        let file_name = Self::domain_to_filename(from_domain);
        let file_path: PathBuf = self.state.lock().unwrap().config_path.join(&file_name);

        fs::write(&file_path, config_content)?;
        Ok(())
    }

    pub fn add_container_host_rule(
        &self,
        domain: &str,
        upstream_url: &str,
        ssl: bool,
        ssl_staging: bool,
        auth: Auth,
        webserver: Webserver,
        protocol: &str,
    ) -> io::Result<()> {
        let _sanitized_domain = sanitize_domain(domain);
        
        // Generate server configuration based on protocol
        let config_content = if protocol == "grpc" {
            generate_grpc_server_block(
                domain,
                upstream_url,
                ssl,
                self.webroot_path.to_str().unwrap_or_default(),
                ssl_staging,
                auth,
                webserver
            )?
        } else {
            // Default to HTTP proxy for http, https, or any other protocol
            generate_proxy_server_block(
                domain,
                upstream_url,
                ssl,
                self.webroot_path.to_str().unwrap_or_default(),
                ssl_staging,
                auth,
                webserver
            )?
        };
        
        // Write server configuration
        let file_name = Self::domain_to_filename(domain);
        let file_path: PathBuf = self.state.lock().unwrap().config_path.join(&file_name);
        fs::write(&file_path, config_content)?;
        
        Ok(())
    }

    pub fn validate_nginx_config(&self) -> io::Result<()> {
        let output = std::process::Command::new("nginx")
            .arg("-t")
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()));
        }
        Ok(())
    }

    pub fn remove_rule(&self, domain: &str) -> io::Result<()> {
        let file_name = Self::domain_to_filename(domain);
        let file_path = self.state.lock().unwrap().config_path.join(&file_name);
        fs::remove_file(&file_path)?;
        Ok(())
    }

    pub fn reload_nginx(&self) -> io::Result<()> {
        let output = std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .output()?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()));
        }
        Ok(())
    }
    
    pub fn check_ssl_certificates_exist(&self, domain: &str, ssl_staging: bool) -> bool {
        let environment = if ssl_staging { "staging" } else { "production" };
        let cert_path = format!("/var/proxma/configuration/{}/live/{}/fullchain.pem", environment, domain);
        let key_path = format!("/var/proxma/configuration/{}/live/{}/privkey.pem", environment, domain);
        Path::new(&cert_path).exists() && Path::new(&key_path).exists()
    }


    /// Remove orphaned config files for domains that no longer have running containers
    pub fn cleanup_orphaned_configs(&self, active_filenames: &[String]) -> io::Result<Vec<String>> {
        let config_path = &self.state.lock().unwrap().config_path;
        let mut removed_domains = Vec::new();
        
        if !config_path.exists() {
            return Ok(removed_domains);
        }
        
        // Read all existing config files
        for entry in fs::read_dir(config_path)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            
            // Check if it's a proxma config file and not in active list
            if filename.starts_with("proxma_") && filename.ends_with(".conf") {
                if !active_filenames.contains(&filename) {
                    // Remove orphaned config file
                    println!("🧹 Removing orphaned config: {}", filename);
                    match fs::remove_file(entry.path()) {
                        Ok(_) => {
                            // Use filename as identifier since we can't reliably reverse sanitize_domain()
                            removed_domains.push(filename.clone());
                            println!("✅ Removed orphaned config: {}", filename);
                        },
                        Err(e) => {
                            eprintln!("❌ Failed to remove {}: {}", filename, e);
                        }
                    }
                }
            }
        }
        
        Ok(removed_domains)
    }
}