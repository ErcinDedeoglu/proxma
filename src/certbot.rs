use std::process::Command;
use std::io::{self, ErrorKind};
use reqwest::blocking::Client;
use std::time::Duration;

#[derive(Debug)]
pub enum CertificateRequestResult {
    Success,
    AcmeChallengeFailure(String), // Combined challenge failures with reason
    CertbotError(String),         // The certbot command itself failed
}

#[derive(Debug)]
pub struct Certbot {
    pub config_dir: Option<String>,
    pub work_dir: Option<String>,
    pub logs_dir: Option<String>,
    pub webroot: String,
    pub email: String,
    pub agree_tos: bool,
    pub staging: bool,
    pub no_eff_email: bool,
}

impl Certbot {
    pub fn new<S: Into<String>>(webroot: S, email: S) -> Self {
        Self {
            webroot: webroot.into(),
            email: email.into(),
            agree_tos: true,
            staging: false,
            no_eff_email: false,
            config_dir: None,
            work_dir: None,
            logs_dir: None,
        }
    }

    pub fn config_dir<S: Into<String>>(mut self, path: S) -> Self {
        self.config_dir = Some(path.into());
        self
    }

    pub fn work_dir<S: Into<String>>(mut self, path: S) -> Self {
        self.work_dir = Some(path.into());
        self
    }

    pub fn logs_dir<S: Into<String>>(mut self, path: S) -> Self {
        self.logs_dir = Some(path.into());
        self
    }

    pub fn agree_tos(mut self, agree_tos: bool) -> Self {
        self.agree_tos = agree_tos;
        self
    }

    pub fn staging(mut self, staging: bool) -> Self {
        self.staging = staging;
        self
    }

    pub fn no_eff_email(mut self, no_eff_email: bool) -> Self {
        self.no_eff_email = no_eff_email;
        self
    }

    pub fn request_certificate<S: AsRef<str>>(&self, domain: S) -> io::Result<()> {
        let mut args = vec![
            "certonly",
            "--non-interactive",
            "--webroot", "-w", &self.webroot,
            "--email", &self.email,
            "--quiet",
            "-d", domain.as_ref()
        ];
    
        if let Some(dir) = &self.config_dir {
            args.push("--config-dir");
            args.push(dir);
        }
        if let Some(dir) = &self.work_dir {
            args.push("--work-dir");
            args.push(dir);
        }
        if let Some(dir) = &self.logs_dir {
            args.push("--logs-dir");
            args.push(dir);
        }
        if self.agree_tos {
            args.push("--agree-tos");
        }
        if self.staging {
            args.push("--staging");
        }
        if self.no_eff_email {
            args.push("--no-eff-email");
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
    
    pub fn check_acme_challenge<S: AsRef<str>>(&self, domain: S) -> io::Result<CertificateRequestResult> {
        let domain_str = domain.as_ref();
        let check_url = format!("http://{}/.well-known/acme-challenge/proxma.proxma", domain_str);
        
        let client: Client = match Client::builder()
            .timeout(Duration::from_secs(10))
            .build() {
                Ok(c) => c,
                Err(e) => return Ok(CertificateRequestResult::AcmeChallengeFailure(
                    format!("Failed to create HTTP client: {}", e)
                )),
        };
        
        let response = match client.get(&check_url).send() {
            Ok(r) => r,
            Err(e) => return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Failed to connect to {}: {}", check_url, e)
            )),
        };
        
        let status = response.status();
        let headers: Vec<(String, String)> = response.headers()
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("invalid utf-8").to_string()))
            .collect();
        
        if !status.is_success() {
            let error_msg = format!(
                "Endpoint {} returned status code: {} {}\nHeaders: {:?}", 
                check_url, status.as_u16(), status.canonical_reason().unwrap_or("Unknown"),
                headers
            );
            return Ok(CertificateRequestResult::AcmeChallengeFailure(error_msg));
        }
        
        let content = match response.text() {
            Ok(t) => t.trim().to_string(),
            Err(e) => return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Failed to read response body from {}: {}", check_url, e)
            )),
        };
        
        if content != "proxma" {
            return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Incorrect content at {}: Expected 'proxma' but got '{}'", check_url, content)
            ));
        }
        
        Ok(CertificateRequestResult::Success)
    }
}