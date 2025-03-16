//! Data models for the application

mod auth;
mod webserver;

// Re-export models
pub use auth::Auth;
pub use webserver::Webserver;
