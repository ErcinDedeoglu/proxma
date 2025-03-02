use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ProxyRule {
    pub id: String,
    pub domains: Vec<String>,
    pub upstream: String,
}

#[derive(Debug)]
pub enum NginxError {
    Io(std::io::Error),
    ReloadFailed(String),
    InvalidRule(String),
}

impl fmt::Display for NginxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NginxError::Io(e) => write!(f, "IO error: {}", e),
            NginxError::ReloadFailed(s) => write!(f, "Nginx reload failed: {}", s),
            NginxError::InvalidRule(s) => write!(f, "Invalid proxy rule: {}", s),
        }
    }
}

impl std::error::Error for NginxError {}

impl From<std::io::Error> for NginxError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct NginxManager {
    rules: Vec<ProxyRule>,
    config_path: PathBuf,
}

impl NginxManager {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            rules: Vec::new(),
            config_path: config_path.as_ref().to_path_buf(),
        }
    }

    pub fn add_rule(&mut self, rule: ProxyRule) -> Result<(), NginxError> {
        // Validate at least one domain exists
        if rule.domains.is_empty() {
            return Err(NginxError::InvalidRule("At least one domain required".into()));
        }

        // Remove existing rules with any overlapping domains
        let new_domains: HashSet<_> = rule.domains.iter().collect();
        self.rules.retain(|existing_rule| {
            !existing_rule.domains.iter().any(|domain| new_domains.contains(domain))
        });

        self.rules.push(rule);
        self.generate_config()?;
        self.reload_nginx()
    }

    pub fn remove_rule_by_id(&mut self, id: &str) -> Result<(), NginxError> {
        let initial_count = self.rules.len();

        // Remove the rule with the matching ID
        self.rules.retain(|r| r.id != id);

        if self.rules.len() != initial_count {
            self.generate_config()?;
            self.reload_nginx()?;
        }
        Ok(())
    }

    pub fn remove_rule_by_domain(&mut self, domain: &str) -> Result<(), NginxError> {
        let initial_count = self.rules.len();

        // Remove any rule containing the specified domain
        self.rules.retain(|r| !r.domains.contains(&domain.to_string()));

        if self.rules.len() != initial_count {
            self.generate_config()?;
            self.reload_nginx()?;
        }
        Ok(())
    }

    fn generate_config(&self) -> Result<(), NginxError> {
        // Begin with http context
        let mut config = String::from("http {\n");
        
        // Add proxy rules
        for rule in &self.rules {
            let server_names = rule.domains.join(" ");
            config.push_str(&format!(
                r#"    server {{
            listen 80;
            server_name {};
        
            location / {{
                proxy_pass {};
                proxy_set_header Host $host;
                proxy_set_header X-Real-IP $remote_addr;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                proxy_set_header X-Forwarded-Proto $scheme;
            }}
        }}
        "#,
                server_names,
                rule.upstream
            ));
        }
        
        // Add default server if no rules exist
        if self.rules.is_empty() {
            config.push_str(
                r#"    server {
            listen 80 default_server;
            return 404;
        }
        "#,
            );
        }
        
        // Close http context
        config.push_str("}\n");
        
        std::fs::write(&self.config_path, config)?;
        Ok(())
    }

    fn reload_nginx(&self) -> Result<(), NginxError> {
        // Test configuration first
        let test_status = std::process::Command::new("nginx")
            .arg("-t")
            .status()?;

        if !test_status.success() {
            return Err(NginxError::ReloadFailed(
                "Configuration test failed. Not reloading.".into(),
            ));
        }

        // Perform actual reload
        let reload_status = std::process::Command::new("nginx")
            .arg("-s")
            .arg("reload")
            .status()?;

        if reload_status.success() {
            Ok(())
        } else {
            Err(NginxError::ReloadFailed(format!(
                "Reload failed with exit code: {}",
                reload_status
            )))
        }
    }
}