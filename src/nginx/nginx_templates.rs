use crate::nginx::nginx_proxy_rule::ProxyRule;

/// Generate an Nginx server block for proxying requests
pub fn generate_proxy_server_block(rule: &ProxyRule, webroot_path: &str) -> String {
    let mut server_blocks = String::new();
    for domain in &rule.domains {
        let ssl_block = if rule.ssl {
            format!(
                r#"    listen 443 ssl;
    ssl_certificate /var/proxma/ssl/default.crt;
    ssl_certificate_key /var/proxma/ssl/default.key;
    location /.well-known/acme-challenge/ {{
        root {};
    }}"#,
                webroot_path
            )
        } else {
            // Add HSTS prevention when SSL is disabled
            r#"    # Prevent browsers from automatically redirecting to HTTPS
    add_header Strict-Transport-Security "max-age=0" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;"#.into()
        };
        server_blocks.push_str(&format!(
            r#"server {{
    listen 80;
{}
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
            ssl_block,
            domain,
            rule.upstream
        ));
    }
    server_blocks
}

/// Generate an Nginx server block for redirecting requests with SSL support
pub fn generate_redirect_server_block(from_domain: &str, to_domain: &str, ssl: bool, webroot_path: &str) -> String {
    let ssl_block = if ssl {
        format!(
            r#"    listen 443 ssl;
    ssl_certificate /var/proxma/ssl/default.crt;
    ssl_certificate_key /var/proxma/ssl/default.key;
    location /.well-known/acme-challenge/ {{
        root {};
    }}"#,
            webroot_path
        )
    } else {
        // Add HSTS prevention when SSL is disabled
        r#"    # Prevent browsers from automatically redirecting to HTTPS
    add_header Strict-Transport-Security "max-age=0" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;"#.into()
    };
    format!(
        r#"server {{
    listen 80;
{}
    server_name {};
    location / {{
        return 301 $scheme://{}$request_uri;
    }}
}}
"#,
        ssl_block,
        from_domain,
        to_domain
    )
}