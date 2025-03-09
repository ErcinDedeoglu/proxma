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
    Dns(String, Option<DnsCredentials>),
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

#[derive(Debug, Clone)]
pub struct DnsCredentials {
    pub(crate) provider: String,
    pub(crate) api_token: String,
    pub(crate) email: Option<String>,
    pub(crate) api_key: Option<String>,
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
    
    pub fn request_certificate<S: AsRef<str>, E: AsRef<str>>(
        &self,
        domain: S,
        ssl_email: E,
        challenge: ChallengeType,
        staging: bool
    ) -> io::Result<()> {
        let mut args = Vec::<String>::new();
        let mut command = Command::new("certbot");
        args.push("certonly".to_string());
        args.push("--non-interactive".to_string());
        args.push("--email".to_string());
        args.push(ssl_email.as_ref().to_string());
        args.push("--quiet".to_string());
        args.push("-d".to_string());
        args.push(domain.as_ref().to_string());
        
        match challenge {
            ChallengeType::Webroot => {
                args.push("--webroot".to_string());
                args.push("-w".to_string());
                args.push(self.webroot.clone());
            },
            ChallengeType::Dns(ref plugin, ref credentials) => {
                if plugin == "cloudflare" {
                    args.push("--dns-cloudflare".to_string());
                    
                    if let Some(creds) = credentials {
                        use std::fs::File;
                        use std::io::Write;
                        use std::path::PathBuf;
                        
                        // Validate credentials
                        if creds.api_token.is_empty() && (creds.email.is_none() || creds.api_key.is_none()) {
                            return Err(io::Error::new(
                                ErrorKind::InvalidInput,
                                "No valid Cloudflare credentials provided"
                            ));
                        }
                        
                        let creds_dir = PathBuf::from(&self.config_dir).join("cloudflare");
                        std::fs::create_dir_all(&creds_dir)?;
                        
                        let creds_file = creds_dir.join("credentials.ini");
                        
                        // Create the credentials file content
                        let creds_content = if !creds.api_token.is_empty() {
                            println!("Using API token authentication");
                            format!(
                                "# Cloudflare API token credentials for Certbot\n\
                                dns_cloudflare_api_token = {}\n",
                                creds.api_token
                            )
                        } else if let (Some(ref email), Some(ref api_key)) = (creds.email.as_ref(), creds.api_key.as_ref()) {
                            println!("Using email/API key authentication");
                            format!(
                                "# Cloudflare API credentials for Certbot\n\
                                dns_cloudflare_email = {}\n\
                                dns_cloudflare_api_key = {}\n",
                                email, api_key
                            )
                        } else {
                            return Err(io::Error::new(
                                ErrorKind::InvalidInput,
                                "Invalid Cloudflare credentials configuration"
                            ));
                        };
                        
                        // Write the credentials file
                        println!("Writing credentials to: {}", creds_file.display());
                        std::fs::write(&creds_file, &creds_content)?;
                        
                        // Set proper permissions
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let mut perms = std::fs::metadata(&creds_file)?.permissions();
                            perms.set_mode(0o600);
                            std::fs::set_permissions(&creds_file, perms)?;
                        }
                        
                        // Verify the file was created and contains the credentials
                        match std::fs::read_to_string(&creds_file) {
                            Ok(contents) => println!("Credentials file contents:\n{}", contents),
                            Err(e) => println!("Failed to read credentials file: {}", e),
                        }
                        
                        args.push("--dns-cloudflare-credentials".to_string());
                        args.push(creds_file.to_string_lossy().to_string());
                    }
                }
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
        
        if matches!(challenge, ChallengeType::Dns(_, _)) {
            args.push("--dns-cloudflare-propagation-seconds".to_string());
            args.push("30".to_string());
        }
        
        println!("Executing command: certbot {}", args.join(" "));
        
        let output = command
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;
        
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            let error_message = format!(
                "Certbot command failed with status: {:?}\nCommand: certbot {}\nOutput: {}\nError: {}", 
                output.status.code(),
                args.join(" "),
                stdout,
                stderr
            );
            
            Err(io::Error::new(ErrorKind::Other, error_message))
        }
    }
}