// Module declarations
mod nginx_manager;
mod nginx_templates;

// Re-export the public items for use outside the module
pub use nginx_manager::NginxManager;