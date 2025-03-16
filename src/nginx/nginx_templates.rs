/// Helper function to generate ACME challenge location block
fn generate_acme_challenge_block(webroot_path: &str) -> String {
    format!(
        r#"    location /.well-known/acme-challenge/ {{
        root {};
    }}"#, webroot_path)
}

/// Helper function to generate a server block with common parameters
fn generate_server_block(is_https: bool, domain: &str, location_block: &str, ssl: bool, webroot_path: Option<&str>, ssl_staging: bool,) -> String {
    let listen_directive = if is_https { "listen 443 ssl;" } else { "listen 80;" };
    let environment = if ssl_staging { "staging" } else { "production" };
    
    let ssl_config = if is_https {
        format!(
            r#"    ssl_certificate /var/proxma/configuration/{environment}/live/{domain}/fullchain.pem;
        ssl_certificate_key /var/proxma/configuration/{environment}/live/{domain}/privkey.pem;"#,
            environment = environment,
            domain = domain
        )
    } else if !ssl {
        // Add HSTS prevention when SSL is disabled
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
    server_name {};
{}
}}
"#,
        listen_directive,
        ssl_config,
        acme_block,
        domain,
        location_block
    )
}

/// Generate an Nginx server block for proxying requests
pub fn generate_proxy_server_block(
    domain: &str, 
    upstream: &str, 
    ssl: bool, 
    webroot_path: &str, 
    ssl_staging: bool,
) -> String {
    let proxy_location = format!(
        r#"    location / {{
        proxy_pass {};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}"#, upstream);
    
    let http_redirect = r#"    location / {
        return 301 https://$host$request_uri;
    }"#;
    
    if ssl {
        format!(
            "# HTTP server for ACME challenges and redirection\n{}\n# HTTPS server for main content\n{}",
            generate_server_block(false, domain, http_redirect, true, Some(webroot_path), ssl_staging),
            generate_server_block(true, domain, &proxy_location, true, None, ssl_staging)
        )
    } else {
        generate_server_block(false, domain, &proxy_location, false, Some(webroot_path), ssl_staging)
    }
}

/// Generate an Nginx server block for redirecting requests with SSL support
pub fn generate_redirect_server_block(from_domain: &str, to_domain: &str, ssl: bool, webroot_path: &str, ssl_staging: bool,) -> String {
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
            generate_server_block(false, from_domain, &http_redirect, true, Some(webroot_path), ssl_staging),
            generate_server_block(true, from_domain, &https_redirect, true, None, ssl_staging)
        )
    } else {
        generate_server_block(false, from_domain, &http_redirect, false, Some(webroot_path), ssl_staging)
    }
}

pub fn sanitize_domain(domain: &str) -> String {
    domain.replace(".", "_").replace("-", "_")
}