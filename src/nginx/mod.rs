// Module declarations
mod nginx_proxy_rule;
mod nginx_error;
mod nginx_manager;
mod nginx_templates;  // Add this line

// Re-export the public items for use outside the module
pub use nginx_proxy_rule::ProxyRule;
pub use nginx_error::NginxError;
pub use nginx_manager::NginxManager;