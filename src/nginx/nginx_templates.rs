use crate::nginx::nginx_proxy_rule::ProxyRule;

/// Generate an Nginx server block for proxying requests
pub fn generate_proxy_server_block(rule: &ProxyRule, webroot_path: &str) -> String {
    let mut server_blocks = String::new();
    
    for domain in &rule.domains {
        server_blocks.push_str(&format!(
            r#"server {{
    listen 80;
    listen 443 ssl;
    server_name {};
    
    # Self-signed certificate (will be replaced by certbot later)
    ssl_certificate /var/proxma/ssl/default.crt;
    ssl_certificate_key /var/proxma/ssl/default.key;
    
    # Serve certbot validation files from given webroot_path
    location /.well-known/acme-challenge/ {{
        root {};
    }}
    
    # Proxy everything else
    location / {{
        proxy_pass {};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}
"#,
            domain,
            webroot_path,
            rule.upstream
        ));
    }
    
    server_blocks
}

/// Generate an Nginx server block for redirecting requests with SSL support
pub fn generate_redirect_server_block(from_domain: &str, to_domain: &str) -> String {
    format!(
        r#"server {{
    listen 80;
    listen 443 ssl;
    server_name {};
    
    # Self-signed certificate (will be replaced by certbot later)
    ssl_certificate /var/proxma/ssl/default.crt;
    ssl_certificate_key /var/proxma/ssl/default.key;
    
    # Allow certbot challenge on redirects too
    location /.well-known/acme-challenge/ {{
        root /var/www/html;
    }}
    
    # Permanent redirect for everything else
    location / {{
        return 301 $scheme://{}$request_uri;
    }}
}}
"#,
        from_domain,
        to_domain
    )
}