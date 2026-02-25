use crate::models::{Auth, AuthType, Webserver};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::net::{TcpListener, SocketAddr};
use bcrypt::{hash, DEFAULT_COST};
use idna::domain_to_ascii;

/// Check if IPv6 is supported on this system
fn is_ipv6_supported() -> bool {
    // Check environment variable override
    if std::env::var("PROXMA_FORCE_IPV6").unwrap_or_default() == "true" {
        return true;
    }
    
    // Try to bind to an IPv6 address
    if let Ok(addr) = "[::1]:0".parse::<SocketAddr>() {
        if TcpListener::bind(addr).is_ok() {
            return true;
        }
    }
    
    // Check if IPv6 is available via /proc/net/if_inet6 (Linux)
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = fs::read_to_string("/proc/net/if_inet6") {
            return !content.trim().is_empty();
        }
    }
    
    false
}

/// Helper function to generate ACME challenge location block
fn generate_acme_challenge_block(webroot_path: &str) -> String {
    format!(
        r#"    location /.well-known/acme-challenge/ {{
        root {};
    }}"#, webroot_path)
}

/// Helper function to generate a server block with common parameters
fn generate_server_block(is_https: bool, domain: &str, location_block: &str, ssl: bool, webroot_path: Option<&str>, ssl_staging: bool, webserver: &Webserver) -> String {
    let ipv6_enabled = is_ipv6_supported();
    
    let listen_directive = if is_https {
        if ipv6_enabled {
            r#"listen 443 ssl;
    listen [::]:443 ssl;
    # Enable HTTP/2
    http2 on;"#
        } else {
            r#"listen 443 ssl;
    # Enable HTTP/2
    http2 on;"#
        }
    } else {
        if ipv6_enabled {
            r#"listen 80;
    listen [::]:80;"#
        } else {
            r#"listen 80;"#
        }
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
    // Only generate file if authentication is enabled and requires basic auth
    if !auth.enabled || auth.auth_type == AuthType::Bearer {
        return Ok(());
    }

    // Create the htpasswd directory if it doesn't exist
    let htpasswd_dir = Path::new("/var/proxma/htpasswd");
    if !htpasswd_dir.exists() {
        fs::create_dir_all(htpasswd_dir)?;
    }

    // Path to the htpasswd file
    let htpasswd_path = htpasswd_dir.join(sanitize_domain(domain));
    
    // Trim whitespace from password to handle newline issues
    let trimmed_password = auth.password.trim();
    
    // Generate bcrypt hash
    let hashed_password = match hash(trimmed_password, DEFAULT_COST) {
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

/// Generate Nginx auth configuration for HTTP/web services
/// Returns directives to be placed INSIDE the location / { } block
fn generate_auth_config(auth: &Auth, domain: &str) -> String {
    if !auth.enabled {
        return String::new();
    }

    match auth.auth_type {
        AuthType::Basic => {
            let htpasswd_file = format!("/var/proxma/htpasswd/{}", sanitize_domain(domain));
            format!(
                r#"
        # Basic auth configuration
        auth_basic "{}";
        auth_basic_user_file {};"#,
                auth.realm,
                htpasswd_file
            )
        },
        AuthType::Bearer => {
            format!(
                r#"
        # Bearer token authentication
        auth_request /_proxma_auth_check;

        # Return 401 with JSON body for failed bearer auth
        error_page 401 = @bearer_unauthorized;"#
            )
        },
        AuthType::Both => {
            let htpasswd_file = format!("/var/proxma/htpasswd/{}", sanitize_domain(domain));
            format!(
                r#"
        # Accept either Basic or Bearer authentication
        satisfy any;
        auth_basic "{}";
        auth_basic_user_file {};
        auth_request /_proxma_auth_check;"#,
                auth.realm,
                htpasswd_file
            )
        },
    }
}

/// Generate Nginx auth configuration specifically for gRPC services
/// This ensures proper gRPC status codes are returned instead of HTML error pages
fn generate_grpc_auth_config(auth: &Auth, domain: &str) -> String {
    if !auth.enabled {
        return String::new();
    }

    // gRPC error page mappings are needed for all auth types
    let grpc_error_pages = r#"
        # Map HTTP auth errors to gRPC-compatible responses
        error_page 401 = @grpc_unauthenticated;
        error_page 403 = @grpc_permission_denied;"#;

    match auth.auth_type {
        AuthType::Basic => {
            let htpasswd_file = format!("/var/proxma/htpasswd/{}", sanitize_domain(domain));
            format!(
                r#"
        # Basic auth configuration for gRPC
        auth_basic "{}";
        auth_basic_user_file {};
        {}"#,
                auth.realm,
                htpasswd_file,
                grpc_error_pages
            )
        },
        AuthType::Bearer => {
            format!(
                r#"
        # Bearer token authentication for gRPC
        auth_request /_proxma_auth_check;
        {}"#,
                grpc_error_pages
            )
        },
        AuthType::Both => {
            let htpasswd_file = format!("/var/proxma/htpasswd/{}", sanitize_domain(domain));
            format!(
                r#"
        # Accept either Basic or Bearer authentication for gRPC
        satisfy any;
        auth_basic "{}";
        auth_basic_user_file {};
        auth_request /_proxma_auth_check;
        {}"#,
                auth.realm,
                htpasswd_file,
                grpc_error_pages
            )
        },
    }
}

/// Generate gRPC error handler locations (must be at server level)
fn generate_grpc_error_handlers(auth: &Auth) -> String {
    if !auth.enabled {
        return String::new();
    }

    // Set www-authenticate header based on auth type
    let www_auth_header = match auth.auth_type {
        AuthType::Basic => format!(
            r#"add_header www-authenticate 'Basic realm="{}"' always;"#,
            auth.realm
        ),
        AuthType::Bearer => r#"add_header www-authenticate 'Bearer' always;"#.to_string(),
        AuthType::Both => format!(
            r#"add_header www-authenticate 'Basic realm="{}", Bearer' always;"#,
            auth.realm
        ),
    };

    format!(
        r#"
    # gRPC UNAUTHENTICATED error handler
    location @grpc_unauthenticated {{
        internal;
        add_header content-type "application/grpc" always;
        add_header grpc-status "16" always;  # UNAUTHENTICATED
        add_header grpc-message "Authentication required" always;
        {}
        return 200;  # gRPC requires HTTP 200 with grpc-status headers
    }}
    
    # gRPC PERMISSION_DENIED error handler
    location @grpc_permission_denied {{
        internal;
        add_header content-type "application/grpc" always;
        add_header grpc-status "7" always;   # PERMISSION_DENIED
        add_header grpc-message "Access denied" always;
        return 200;  # gRPC requires HTTP 200 with grpc-status headers
    }}"#,
        www_auth_header
    )
}

/// Generate the bearer token validation internal location block
/// Must be placed at server level (sibling to location /)
fn generate_bearer_validation_location(auth: &Auth) -> String {
    if !auth.enabled {
        return String::new();
    }

    match auth.auth_type {
        AuthType::Bearer | AuthType::Both => {
            format!(
                r#"

    # Internal endpoint for bearer token validation
    location = /_proxma_auth_check {{
        internal;
        if ($http_authorization != "Bearer {}") {{
            return 401;
        }}
        return 200;
    }}"#,
                auth.token
            )
        },
        AuthType::Basic => String::new(),
    }
}

/// Generate the bearer unauthorized error handler for HTTP/web services
/// Returns a JSON 401 response for bearer-only auth failures
fn generate_bearer_unauthorized_location(auth: &Auth) -> String {
    if !auth.enabled {
        return String::new();
    }

    // Only needed for bearer-only mode (both mode falls back to basic auth prompt)
    if auth.auth_type == AuthType::Bearer {
        r#"

    # Bearer auth 401 response handler
    location @bearer_unauthorized {
        internal;
        default_type application/json;
        add_header WWW-Authenticate 'Bearer' always;
        return 401 '{"error": "Unauthorized", "message": "Valid Bearer token required"}';
    }"#.to_string()
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
        resolver 127.0.0.11 valid=10s;
        set $proxma_upstream {};
        proxy_pass $proxma_upstream;
        
        # Forward ALL headers by default
        proxy_pass_request_headers on;
        
        # Standard proxy headers
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
    
    // Generate bearer auth locations (server-level, sibling to location /)
    let bearer_validation = generate_bearer_validation_location(&auth);
    let bearer_unauthorized = generate_bearer_unauthorized_location(&auth);
    let full_location_block = format!("{}{}{}", proxy_location, bearer_validation, bearer_unauthorized);

    let http_redirect = r#"    location / {
        return 301 https://$host$request_uri;
    }"#;
    
    let result = if ssl {
        format!(
            "# HTTP server for ACME challenges and redirection\n{}\n# HTTPS server for main content\n{}",
            generate_server_block(false, domain, http_redirect, true, Some(webroot_path), ssl_staging, &webserver),
            generate_server_block(true, domain, &full_location_block, true, None, ssl_staging, &webserver)
        )
    } else {
        generate_server_block(false, domain, &full_location_block, false, Some(webroot_path), ssl_staging, &webserver)
    };
    
    Ok(result)
}

/// Generate an Nginx server block for gRPC proxying requests
pub fn generate_grpc_server_block(
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
    
    // Get gRPC-specific auth configuration
    let auth_config = generate_grpc_auth_config(&auth, domain);
    
    // Add auth_config to the grpc_location
    let (xfwd_port, xfwd_proto) = if ssl {
        ("443", "https")
    } else {
        ("80", "http")
    };
    let grpc_location = format!(
        r#"    location / {{{}
        resolver 127.0.0.11 valid=10s;
        set $proxma_upstream {};
        grpc_pass grpc://$proxma_upstream;
        grpc_read_timeout {};
        grpc_send_timeout {};
        grpc_connect_timeout {};
        
        # Additional gRPC error handling for proxy errors
        error_page 502 = @grpc_unavailable;
        error_page 503 = @grpc_unavailable;
        error_page 504 = @grpc_deadline_exceeded;
        error_page 404 = @grpc_unimplemented;
        
        # gRPC automatically forwards all headers by default
        # Standard gRPC headers
        grpc_set_header Host $host;
        grpc_set_header X-Forwarded-Host $host;
        grpc_set_header X-Forwarded-Port {};
        grpc_set_header X-Forwarded-Proto {};
        grpc_set_header X-Real-IP $remote_addr;
        grpc_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        grpc_set_header X-Forwarded-Scheme $scheme;
        grpc_set_header X-Original-URI $request_uri;
        grpc_set_header X-Forwarded-Server $host;
        grpc_set_header X-Request-Start $msec;
        grpc_set_header X-Original-Host $host;
        grpc_set_header X-Forwarded-SSL on;
    }}"#,
        auth_config,
        upstream,
        webserver.proxy_read_timeout,
        webserver.proxy_send_timeout,
        webserver.proxy_connect_timeout,
        xfwd_port,
        xfwd_proto
    );
    
    // Generate all gRPC error handlers at server level
    let grpc_error_handlers = format!(
        r#"{}
    
    # gRPC UNAVAILABLE error handler (for 502/503 errors)
    location @grpc_unavailable {{
        internal;
        add_header content-type "application/grpc" always;
        add_header grpc-status "14" always;  # UNAVAILABLE
        add_header grpc-message "Service unavailable" always;
        return 200;  # gRPC requires HTTP 200 with grpc-status headers
    }}
    
    # gRPC DEADLINE_EXCEEDED error handler (for 504 timeout errors)
    location @grpc_deadline_exceeded {{
        internal;
        add_header content-type "application/grpc" always;
        add_header grpc-status "4" always;   # DEADLINE_EXCEEDED
        add_header grpc-message "Request timeout" always;
        return 200;  # gRPC requires HTTP 200 with grpc-status headers
    }}
    
    # gRPC UNIMPLEMENTED error handler (for 404 errors)
    location @grpc_unimplemented {{
        internal;
        add_header content-type "application/grpc" always;
        add_header grpc-status "12" always;  # UNIMPLEMENTED
        add_header grpc-message "Method not found" always;
        return 200;  # gRPC requires HTTP 200 with grpc-status headers
    }}"#,
        generate_grpc_error_handlers(&auth)
    );
    
    // Generate bearer auth validation location (server-level, sibling to other locations)
    let bearer_validation = generate_bearer_validation_location(&auth);
    let full_location_block = format!("{}{}{}", grpc_location, grpc_error_handlers, bearer_validation);

    let http_redirect = r#"    location / {
        return 301 https://$host$request_uri;
    }"#;
    
    let result = if ssl {
        format!(
            "# HTTP server for ACME challenges and redirection\n{}\n# HTTPS server for gRPC content\n{}",
            generate_server_block(false, domain, http_redirect, true, Some(webroot_path), ssl_staging, &webserver),
            generate_server_block(true, domain, &full_location_block, true, None, ssl_staging, &webserver)
        )
    } else {
        generate_server_block(false, domain, &full_location_block, false, Some(webroot_path), ssl_staging, &webserver)
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
