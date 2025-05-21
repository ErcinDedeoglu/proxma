use crate::models::{Auth, Webserver};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use bcrypt::{hash, DEFAULT_COST};
use idna::domain_to_ascii;

/// Helper function to generate ACME challenge location block
fn generate_acme_challenge_block(webroot_path: &str) -> String {
    format!(
        r#"    location /.well-known/acme-challenge/ {{
        root {};
    }}"#, webroot_path)
}

/// Helper function to generate a server block with common parameters
fn generate_server_block(is_https: bool, domain: &str, location_block: &str, ssl: bool, webroot_path: Option<&str>, ssl_staging: bool, webserver: &Webserver) -> String {
    let listen_directive = if is_https {
        r#"listen 443 ssl;
    listen [::]:443 ssl;
    # Enable HTTP/2
    http2 on;"#
    } else {
        r#"listen 80;
    listen [::]:80;"#
    };
    let environment = if ssl_staging { "staging" } else { "production" };
    
    // Add all webserver configuration directives
    let timeout_directives = format!(
        "    client_max_body_size {};\n\
        client_body_timeout {};\n\
        client_header_timeout {};\n\
        send_timeout {};\n\
        keepalive_timeout {};\n\
        proxy_connect_timeout {};\n\
        proxy_send_timeout {};\n\
        proxy_read_timeout {};",
        webserver.client_max_body_size,
        webserver.client_body_timeout,
        webserver.client_header_timeout,
        webserver.send_timeout,
        webserver.keepalive_timeout,
        webserver.proxy_connect_timeout,
        webserver.proxy_send_timeout,
        webserver.proxy_read_timeout
    );
    
    let ssl_config = if is_https {
        format!(
            r#"    ssl_certificate /var/proxma/configuration/{environment}/live/{domain}/fullchain.pem;
    ssl_certificate_key /var/proxma/configuration/{environment}/live/{domain}/privkey.pem;

    # SSL settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers off;"#,
            environment = environment,
            domain = domain
        )
    } else if !ssl {
        r#"    # Prevent browsers from automatically redirecting to HTTPS
    add_header Strict-Transport-Security "max-age=0" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;"#.into()
    } else {
        String::new()
    };
    
    let acme_block = if let Some(path) = webroot_path {
        generate_acme_challenge_block(path)
    } else {
        String::new()
    };
    
    format!(
        r#"server {{
    {}
    {}
{}

{}
    server_name {};
{}
}}
"#,
        listen_directive,
        timeout_directives,
        ssl_config,
        acme_block,
        domain,
        location_block
    )
}

/// Generate an htpasswd file for basic authentication
pub fn generate_htpasswd_file(auth: &Auth, domain: &str) -> io::Result<()> {
    // Only generate file if authentication is enabled
    if !auth.enabled {
        return Ok(());
    }

    // Create the htpasswd directory if it doesn't exist
    let htpasswd_dir = Path::new("/var/proxma/htpasswd");
    if !htpasswd_dir.exists() {
        fs::create_dir_all(htpasswd_dir)?;
    }

    // Path to the htpasswd file
    let htpasswd_path = htpasswd_dir.join(sanitize_domain(domain));
    
    // Generate bcrypt hash
    let hashed_password = match hash(&auth.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
    };
    
    // Format: username:hashed_password
    let htpasswd_content = format!("{}:{}\n", auth.username, hashed_password);
    
    // Write to the file
    let mut file = File::create(&htpasswd_path)?;
    file.write_all(htpasswd_content.as_bytes())?;
    
    Ok(())
}

/// Generate Nginx basic auth configuration
fn generate_auth_config(auth: &Auth, domain: &str) -> String {
    if auth.enabled {
        let htpasswd_file = format!("/var/proxma/htpasswd/{}", sanitize_domain(domain));
        
        format!(
            r#"
        # Basic auth configuration
        auth_basic "{}";
        auth_basic_user_file {};
        
        # Ensure browsers re-prompt for credentials
        error_page 401 403 =401 /401_error;
        location = /401_error {{
            internal;
            add_header WWW-Authenticate 'Basic realm="{}";' always;
            add_header Cache-Control "no-store, no-cache, must-revalidate" always;
            add_header Pragma "no-cache" always;
            return 401;
        }}"#,
            auth.realm,
            htpasswd_file,
            auth.realm
        )
    } else {
        String::new()
    }
}

/// Generate an Nginx server block for proxying requests
pub fn generate_proxy_server_block(
    domain: &str, 
    upstream: &str, 
    ssl: bool, 
    webroot_path: &str, 
    ssl_staging: bool,
    auth: Auth,
    webserver: Webserver,
) -> io::Result<String> {
    // Generate htpasswd file if authentication is enabled
    generate_htpasswd_file(&auth, domain)?;
    
    // Get auth configuration
    let auth_config = generate_auth_config(&auth, domain);
    
    // Add auth_config to the proxy_location
    let (xfwd_port, xfwd_proto) = if ssl {
        ("443", "https")
    } else {
        ("80", "http")
    };
    let proxy_location = format!(
        r#"    location / {{{}
        proxy_pass {};
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header X-Forwarded-Port {};
        proxy_set_header X-Forwarded-Proto {};
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Scheme $scheme;
        proxy_set_header X-Original-URI $request_uri;
        proxy_set_header X-Forwarded-Server $host;
        proxy_set_header X-Request-Start $msec;
        proxy_set_header X-Original-Host $host;
        proxy_set_header X-Forwarded-SSL on;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;  # Longer timeout for WebSockets
    }}"#,
        auth_config,
        upstream,
        xfwd_port,
        xfwd_proto
    );
    
    let http_redirect = r#"    location / {
        return 301 https://$host$request_uri;
    }"#;
    
    let result = if ssl {
        format!(
            "# HTTP server for ACME challenges and redirection\n{}\n# HTTPS server for main content\n{}",
            generate_server_block(false, domain, http_redirect, true, Some(webroot_path), ssl_staging, &webserver),
            generate_server_block(true, domain, &proxy_location, true, None, ssl_staging, &webserver)
        )
    } else {
        generate_server_block(false, domain, &proxy_location, false, Some(webroot_path), ssl_staging, &webserver)
    };
    
    Ok(result)
}

/// Generate an Nginx server block for redirecting requests with SSL support
pub fn generate_redirect_server_block(
    from_domain: &str, 
    to_domain: &str, 
    ssl: bool, 
    webroot_path: &str, 
    ssl_staging: bool,
    webserver: Webserver,
) -> String {
    // Rest of the function remains the same, just update the calls to generate_server_block
    let http_redirect = format!(
        r#"    location / {{
        return 301 {}://{}$request_uri;
    }}"#, if ssl { "https" } else { "$scheme" }, to_domain);
    
    let https_redirect = format!(
        r#"    location / {{
        return 301 https://{}$request_uri;
    }}"#, to_domain);
    
    if ssl {
        format!(
            "# HTTP server for ACME challenges and redirection\n{}\n# HTTPS server for redirection\n{}",
            generate_server_block(false, from_domain, &http_redirect, true, Some(webroot_path), ssl_staging, &webserver),
            generate_server_block(true, from_domain, &https_redirect, true, None, ssl_staging, &webserver)
        )
    } else {
        generate_server_block(false, from_domain, &http_redirect, false, Some(webroot_path), ssl_staging, &webserver)
    }
}

pub fn sanitize_domain(domain: &str) -> String {
    // Convert to lowercase
    let domain = domain.to_lowercase();
    
    // Strip protocol prefixes if present
    let domain = domain.trim_start_matches("http://")
                      .trim_start_matches("https://")
                      .trim_start_matches("ftp://");
    
    // Remove path, query parameters, and fragments
    let domain = domain.split('/').next().unwrap_or(domain);
    let domain = domain.split('?').next().unwrap_or(domain);
    let domain = domain.split('#').next().unwrap_or(domain);
    
    // Handle IDNs by converting to Punycode (ASCII representation)
    let ascii_domain = match domain_to_ascii(domain) {
        Ok(ascii) => ascii,
        Err(_) => domain.to_string(), // Fallback if conversion fails
    };
    
    // Replace problematic characters with underscores
    ascii_domain.replace(".", "_").replace("-", "_")
}
