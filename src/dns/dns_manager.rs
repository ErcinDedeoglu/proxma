use lazy_static::lazy_static;

use super::CloudflareManager;

pub struct DNSManager {
}

lazy_static! {
    pub static ref CLOUDFLARE_MANAGER: CloudflareManager = CloudflareManager::new();
}

impl DNSManager {
    pub fn new() -> Self {
        DNSManager {}
    }

    pub async fn add_update_record(&self, provider: String, record: String, target: String, r#type: String, proxied: bool, email: String, api_key: String, api_token: String) -> bool {
        println!("🔒 Adding DNS record for '{}' with target '{}', type '{}', provider '{}', proxied: {}", record, target, r#type, provider, proxied);

        if provider == "cloudflare" {
            CLOUDFLARE_MANAGER.add_update_record(provider, record, target, r#type, proxied, email, api_key, api_token).await;
        }
        
        true
    }
}