use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::nginx::nginx_error::NginxError;
use crate::nginx::nginx_proxy_rule::ProxyRule;
use crate::nginx::nginx_templates::{generate_proxy_server_block, generate_redirect_server_block};

pub struct NginxManager {
    rules: Vec<ProxyRule>,
    config_path: PathBuf,
    webroot_path: PathBuf,
}

impl NginxManager {
    pub fn new<P: AsRef<Path>, W: AsRef<Path>>(config_path: P, webroot_path: W) -> Self {
        Self {
            rules: Vec::new(),
            config_path: config_path.as_ref().to_path_buf(),
            webroot_path: webroot_path.as_ref().to_path_buf(),
        }
    }
    
    pub fn add_rule(&mut self, mut rule: ProxyRule) -> Result<(), NginxError> {
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
        self.rules.retain(|existing_rule| {
            let existing_primary = existing_rule.domains.iter().cloned().collect::<HashSet<_>>();
            let existing_redirects = existing_rule.redirects.iter()
                .map(|(from, _)| from.clone())
                .collect::<HashSet<_>>();
            let existing_all = existing_primary.union(&existing_redirects).cloned().collect::<HashSet<_>>();
            
            existing_all.is_disjoint(&all_domains)
        });
        
        self.rules.push(rule);
        self.generate_config()?;
        self.reload_nginx()
    }
    
    pub fn remove_rule_by_id(&mut self, id: &str) -> Result<(), NginxError> {
        let initial_count = self.rules.len();
        self.rules.retain(|r| r.id != id);
        if self.rules.len() != initial_count {
            self.generate_config()?;
            self.reload_nginx()?;
        }
        Ok(())
    }
    
    pub fn remove_rule_by_domain(&mut self, domain: &str) -> Result<(), NginxError> {
        let initial_count = self.rules.len();
        self.rules.retain(|r| !r.domains.contains(&domain.to_string()));
        if self.rules.len() != initial_count {
            self.generate_config()?;
            self.reload_nginx()?;
        }
        Ok(())
    }
    
    fn generate_config(&self) -> Result<(), NginxError> {
        let mut config = String::new();
        
        for rule in &self.rules {
            config.push_str(&generate_proxy_server_block(
                rule, 
                &self.webroot_path.to_string_lossy()
            ));
    
            for (from_domain, to_domain) in &rule.redirects {
                config.push_str(&generate_redirect_server_block(from_domain, to_domain));
            }
        }
        
        std::fs::write(&self.config_path, config)?;
        Ok(())
    }
    
    fn reload_nginx(&self) -> Result<(), NginxError> {
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
}