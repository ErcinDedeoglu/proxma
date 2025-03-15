use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, io};
use crate::nginx::nginx_templates::generate_proxy_server_block;
use crate::nginx::nginx_templates::generate_redirect_server_block;
use crate::models::Cache;

use super::nginx_templates::{generate_cache_zone_file, sanitize_domain};

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

    pub fn add_redirect_rule(
        &self,
        from_domain: &str,
        to_domain: &str,
        ssl: bool,
        ssl_staging: bool,
    ) -> io::Result<()> {
        let config_content = generate_redirect_server_block(
            from_domain,
            to_domain,
            ssl,
            self.webroot_path.to_str().unwrap_or_default(),
            ssl_staging
        );

        let file_name = format!("proxma_{}.conf", from_domain.replace('.', "_"));
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
        cache: Cache,
    ) -> io::Result<()> {
        let sanitized_domain = sanitize_domain(domain);

        if cache.enabled {
            // Generate cache zone configuration        
            let cache_zone_content: String = generate_cache_zone_file(domain, cache.clone());
        
            // Create cache zone directory
            let cache_zones_dir = Path::new("/etc/nginx/cache-zones.d");
            if !cache_zones_dir.exists() {
                fs::create_dir_all(cache_zones_dir)?;
            }
            
            // Write cache zone configuration
            fs::write(cache_zones_dir.join(format!("{}.conf", sanitized_domain)), cache_zone_content)?;
        
            // Create cache directory
            let cache_name = format!("{}_cache", sanitized_domain);
            let cache_dir: PathBuf = Path::new("/var/cache/nginx").join(&cache_name);
            if !cache_dir.exists() {
                fs::create_dir_all(&cache_dir)?;
            }
        }
        
        // Generate server configuration
        let config_content = generate_proxy_server_block(
            domain,
            upstream_url,
            ssl,
            self.webroot_path.to_str().unwrap_or_default(),
            ssl_staging,
            cache,
        );
    
        // Write server configuration
        let file_name = format!("proxma_{}.conf", sanitized_domain);
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
        let file_name = format!("proxma_{}.conf", domain.replace('.', "_"));
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
}