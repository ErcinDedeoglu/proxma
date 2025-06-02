use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref ACTIVE_DOMAINS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub struct DomainTracker;

impl DomainTracker {
    /// Add a domain to the active list (converts to filename internally)
    pub fn add_domain(original_domain: String) {
        use crate::nginx::nginx_manager::NginxManager;
        let filename = NginxManager::domain_to_filename(&original_domain);
        let mut domains = ACTIVE_DOMAINS.lock().unwrap();
        domains.insert(filename, original_domain);
    }

    /// Remove a domain from the active list (converts to filename internally)
    pub fn remove_domain(original_domain: &str) {
        use crate::nginx::nginx_manager::NginxManager;
        let filename = NginxManager::domain_to_filename(original_domain);
        let mut domains = ACTIVE_DOMAINS.lock().unwrap();
        domains.remove(&filename);
    }

    /// Get all active filenames
    pub fn get_active_filenames() -> Vec<String> {
        let domains = ACTIVE_DOMAINS.lock().unwrap();
        domains.keys().cloned().collect()
    }


    /// Clean up orphaned configs by comparing with disk
    pub fn cleanup_orphaned_configs() -> Result<Vec<String>, std::io::Error> {
        use crate::queue::nginx_queue_processor::NGINX_MANAGER;
        
        let active_filenames = Self::get_active_filenames();
        println!("📋 Active domains in memory: {} domains", active_filenames.len());
        
        NGINX_MANAGER.cleanup_orphaned_configs(&active_filenames)
    }
}