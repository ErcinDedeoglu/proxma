use crate::nginx::nginx_proxy_rule::ProxyRule;

/// Generate an Nginx server block for proxying requests
pub fn generate_proxy_server_block(rule: &ProxyRule, webroot_path: &str) -> String {
    let server_names = rule.domains.join(" ");
    format!(
        r#"server {{
    listen 80;
    server_name {};

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
        server_names,
        webroot_path,
        rule.upstream
    )
}

/// Generate an Nginx server block for redirecting requests
pub fn generate_redirect_server_block(from_domain: &str, to_domain: &str) -> String {
    format!(
        r#"server {{
    listen 80;
    server_name {};
    
    # Permanent redirect
    return 301 $scheme://{}$request_uri;
}}
"#,
        from_domain,
        to_domain
    )
}