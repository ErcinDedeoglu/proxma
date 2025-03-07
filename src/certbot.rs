use std::process::Command;
use std::io::{self, ErrorKind};

#[derive(Debug)]
pub enum CertificateRequestResult {
    Success,
    AcmeChallengeFailure(String),
    CertbotError(String),
}

#[derive(Debug, Clone)]
pub enum ChallengeType {
    Webroot,
    Dns(String),
}

#[derive(Debug)]
pub struct Certbot {
    webroot: String,
    config_dir: String,
    work_dir: String,
    logs_dir: String,
    agree_tos: bool,
    no_eff_email: bool,
}

impl Certbot {
    pub fn new() -> Self {
        Self {
            webroot: "/var/www/html".to_string(),
            config_dir: "/var/proxma/configuration".to_string(),
            work_dir: "/var/proxma/work".to_string(),
            logs_dir: "/var/proxma/logs".to_string(),
            agree_tos: true,
            no_eff_email: false,
        }
    }
    
    pub fn request_certificate<S: AsRef<str>, E: AsRef<str>>(&self, domain: S, email: E, challenge: ChallengeType, staging: bool) -> io::Result<()> {
        let mut args = Vec::<String>::new();
        
        args.push("certonly".to_string());
        args.push("--non-interactive".to_string());
        args.push("--email".to_string());
        args.push(email.as_ref().to_string());
        args.push("--quiet".to_string());
        args.push("-d".to_string());
        args.push(domain.as_ref().to_string());
        
        match challenge {
            ChallengeType::Webroot => {
                args.push("--webroot".to_string());
                args.push("-w".to_string());
                args.push(self.webroot.clone());
            },
            ChallengeType::Dns(plugin) => {
                args.push(format!("--dns-{}", plugin));
            }
        }

        args.push("--config-dir".to_string());
        args.push(self.config_dir.clone());
        args.push("--work-dir".to_string());
        args.push(self.work_dir.clone());
        args.push("--logs-dir".to_string());
        args.push(self.logs_dir.clone());
        
        if self.agree_tos {
            args.push("--agree-tos".to_string());
        }
        if staging {
            args.push("--staging".to_string());
        }
        if self.no_eff_email {
            args.push("--no-eff-email".to_string());
        }

        let output = Command::new("certbot")
            .args(&args)
            .stdout(std::process::Stdio::null())  
            .stderr(std::process::Stdio::null())
            .status()?;

        if output.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                ErrorKind::Other,
                format!("Certbot command exited with status: {:?}", output.code()),
            ))
        }
    }
}