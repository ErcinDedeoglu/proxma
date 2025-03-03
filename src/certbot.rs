use std::process::Command;
use std::io::{self, ErrorKind};

#[derive(Debug)]
pub struct Certbot {
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
            agree_tos: true, // default recommended true
            staging: false,  // default production mode
            no_eff_email: false, // default allow LE emails
        }
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

        let status = Command::new("sudo")
            .arg("certbot")
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
}