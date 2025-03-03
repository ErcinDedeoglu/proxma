use std::fs;
use std::os::unix::fs as unix_fs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use crate::nginx::nginx_error::NginxError;
use crate::nginx::nginx_proxy_rule::ProxyRule;
use crate::nginx::nginx_templates::{generate_proxy_server_block, generate_redirect_server_block};

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
    
    pub fn add_rule(&self, mut rule: ProxyRule) -> Result<(), NginxError> {
        // Lock the entire state for the duration of the method
        let mut state = self.state.lock()
            .map_err(|_| NginxError::InvalidRule("Failed to acquire state lock".into()))?;
        
        if rule.domains.is_empty() {
            return Err(NginxError::InvalidRule("At least one domain required".into()));
        }
        
        let redirect_sources: HashSet<_> = rule.redirects.iter()
            .map(|(from, _)| from.clone())
            .collect();
        
        rule.domains = rule.domains.into_iter()
            .filter(|d| !redirect_sources.contains(d))
            .collect();
        
        if rule.domains.is_empty() {
            return Err(NginxError::InvalidRule(
                "All primary domains are used as redirect sources. At least one primary domain is required.".into()
            ));
        }
        
        let primary_domains: HashSet<_> = rule.domains.iter().cloned().collect();
        for (_, to_domain) in &rule.redirects {
            if !primary_domains.contains(to_domain) {
                return Err(NginxError::InvalidRule(
                    format!("Redirect target '{}' must be one of the primary domains", to_domain)
                ));
            }
        }
        
        let all_domains: HashSet<_> = primary_domains.union(&redirect_sources).cloned().collect();
        state.rules.retain(|existing_rule| {
            let existing_primary = existing_rule.domains.iter().cloned().collect::<HashSet<_>>();
            let existing_redirects = existing_rule.redirects.iter()
                .map(|(from, _)| from.clone())
                .collect::<HashSet<_>>();
            let existing_all = existing_primary.union(&existing_redirects).cloned().collect::<HashSet<_>>();
            existing_all.is_disjoint(&all_domains)
        });
        
        state.rules.push(rule.clone()); // Clone the rule to use it after releasing the lock
        
        // Generate config while still holding the lock
        let mut config = String::new();
        for rule in &state.rules {
            config.push_str(&generate_proxy_server_block(
                rule,
                &self.webroot_path.to_string_lossy()
            ));
            for (from_domain, to_domain) in &rule.redirects {
                config.push_str(&generate_redirect_server_block(
                    from_domain,
                    to_domain,
                    rule.ssl,
                    &self.webroot_path.to_string_lossy(),
                ));
            }
        }
        
        // Write config file while still holding the lock
        std::fs::write(&state.config_path, config)?;
    
        // Create certificate links before reloading Nginx
        if rule.ssl {
            self.create_default_certificate_links(&rule)?;
        }
        
        // Reload nginx while still holding the lock
        self.reload_nginx()
    }
    
    pub fn remove_rule_by_id(&self, id: &str) -> Result<(), NginxError> {
        // Lock the entire state for the duration of the method
        let mut state = self.state.lock()
            .map_err(|_| NginxError::InvalidRule("Failed to acquire state lock".into()))?;
            
        let initial_count = state.rules.len();
        state.rules.retain(|r| r.id != id);
        
        if state.rules.len() != initial_count {
            // Generate config while still holding the lock
            let mut config = String::new();
            for rule in &state.rules {
                config.push_str(&generate_proxy_server_block(
                    rule, 
                    &self.webroot_path.to_string_lossy()
                ));
        
                for (from_domain, to_domain) in &rule.redirects {
                    config.push_str(&generate_redirect_server_block(
                        from_domain,
                        to_domain,
                        rule.ssl,
                        &self.webroot_path.to_string_lossy(),
                    ));
                }
            }
            
            // Write config file while still holding the lock
            std::fs::write(&state.config_path, config)?;
            
            // Reload nginx while still holding the lock
            self.reload_nginx()?;
        }
        
        Ok(())
    }
    
    pub fn remove_rule_by_domain(&self, domain: &str) -> Result<(), NginxError> {
        // Lock the entire state for the duration of the method
        let mut state = self.state.lock()
            .map_err(|_| NginxError::InvalidRule("Failed to acquire state lock".into()))?;
            
        let initial_count = state.rules.len();
        state.rules.retain(|r| !r.domains.contains(&domain.to_string()));
        
        if state.rules.len() != initial_count {
            // Generate config while still holding the lock
            let mut config = String::new();
            for rule in &state.rules {
                config.push_str(&generate_proxy_server_block(
                    rule, 
                    &self.webroot_path.to_string_lossy()
                ));
        
                for (from_domain, to_domain) in &rule.redirects {
                    config.push_str(&generate_redirect_server_block(
                        from_domain,
                        to_domain,
                        rule.ssl,
                        &self.webroot_path.to_string_lossy(),
                    ));
                }
            }
            
            // Write config file while still holding the lock
            std::fs::write(&state.config_path, config)?;
            
            // Reload nginx while still holding the lock
            self.reload_nginx()?;
        }
        
        Ok(())
    }
    
    fn reload_nginx(&self) -> Result<(), NginxError> {
        // Note: we're still holding the state lock while this executes
        let status_check = std::process::Command::new("sh")
            .arg("-c")
            .arg("nginx -t 2>/dev/null || echo 'not running'")
            .output()?;
        
        let is_running = !String::from_utf8_lossy(&status_check.stdout).contains("not running");
        let test_output = std::process::Command::new("nginx")
            .arg("-t")
            .output()?;
        
        if !test_output.status.success() {
            let error = String::from_utf8_lossy(&test_output.stderr);
            return Err(NginxError::ReloadFailed(format!("Config test failed: {}", error)));
        }
        
        let reload_cmd = if is_running { "nginx -s reload" } else { "nginx" };
        let reload_output = std::process::Command::new("sh")
            .arg("-c")
            .arg(reload_cmd)
            .output()?;
        
        if reload_output.status.success() {
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&reload_output.stderr);
            Err(NginxError::ReloadFailed(format!("Nginx {} failed: {}", 
                if is_running { "reload" } else { "start" }, error)))
        }
    }

    fn create_default_certificate_links(&self, rule: &ProxyRule) -> Result<(), NginxError> {
        // Define default certificate paths
        let default_cert_dir = Path::new("/var/proxma/ssl");
        let default_fullchain = default_cert_dir.join("default.fullchain.crt.pem");
        let default_privkey = default_cert_dir.join("default.privkey.key.pem");
        
        // Create the default certificate directory if it doesn't exist
        if !default_cert_dir.exists() {
            fs::create_dir_all(default_cert_dir)
                .map_err(|e| NginxError::Io(e))?;
        }
        
        // Only proceed if links don't already exist
        if default_fullchain.exists() && default_privkey.exists() {
            return Ok(());
        }
        
        // Handle both primary domains and redirect domains
        let mut all_domains = rule.domains.clone();
        let redirect_domains: Vec<String> = rule.redirects.iter()
            .map(|(from, _)| from.clone())
            .collect();
        all_domains.extend(redirect_domains);
        
        // Try each domain until we find one with valid certificates
        for domain in all_domains {
            let domain_cert_dir = format!("/var/proxma/letsencrypt/live/{}", domain);
            let domain_fullchain = Path::new(&domain_cert_dir).join("fullchain.pem");
            let domain_privkey = Path::new(&domain_cert_dir).join("privkey.pem");
            
            // Skip if certificates don't exist for this domain
            if !domain_fullchain.exists() || !domain_privkey.exists() {
                continue;
            }
            
            // Create symlinks if destination files don't exist
            if !default_fullchain.exists() {
                unix_fs::symlink(&domain_fullchain, &default_fullchain)
                    .map_err(|e| NginxError::Io(e))?;
                println!("Created certificate symlink: {} -> {}", 
                    default_fullchain.display(), domain_fullchain.display());
            }
            
            if !default_privkey.exists() {
                unix_fs::symlink(&domain_privkey, &default_privkey)
                    .map_err(|e| NginxError::Io(e))?;
                println!("Created key symlink: {} -> {}", 
                    default_privkey.display(), domain_privkey.display());
            }
            
            // If we got here, we successfully handled at least one domain
            return Ok(());
        }
        
        // If no domain had valid certificates, log a warning but don't fail
        println!("Warning: No valid certificates found for any domains in this rule");
        Ok(())
    }
}