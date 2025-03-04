use std::process::Command;
use std::io::{self, ErrorKind};

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

    pub fn request_certificate<S: AsRef<str>>(&self, domains: &[S]) -> io::Result<()> {
        let mut args = vec![
            "certonly",
            "--non-interactive",
            "--webroot", "-w", &self.webroot,
            "--email", &self.email,
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

        for domain in domains {
            args.push("-d");
            args.push(domain.as_ref());
        }

        let status = Command::new("certbot")
        .args(&args)
        .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                ErrorKind::Other,
                format!("Certbot command exited with status: {:?}", status.code()),
            ))
        }
    }

    pub fn check_and_request_certificate<S: AsRef<str>>(&self, domain: S) -> io::Result<CertificateRequestResult> {
        let domain_str = domain.as_ref();
        let check_url = format!("http://{}/.well-known/acme-challenge/proxma.proxma", domain_str);
        
        // Use curl to check the challenge endpoint
        let output = Command::new("curl")
            .arg("--silent")
            .arg("--show-error")
            .arg("--fail")
            .arg("--max-time")
            .arg("10")  // 10 second timeout
            .arg(&check_url)
            .output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Endpoint unreachable: {}", error_msg.trim())
            ));
        }
        
        let content = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if content != "proxma" {
            return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Incorrect content: '{}'", content)
            ));
        }
        
        // Challenge verification succeeded, request certificate
        match self.request_certificate(&[domain_str]) {
            Ok(_) => Ok(CertificateRequestResult::Success),
            Err(e) => Ok(CertificateRequestResult::CertbotError(e.to_string()))
        }
    }
}