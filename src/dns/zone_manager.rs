use cloudflare::framework::client::async_api::Client as HttpApiClientAsync;
use cloudflare::framework::{Environment, response::ApiFailure};
use cloudflare::endpoints::zones::zone::{ListZones, ListZonesParams};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;
use super::auth_manager::AuthManager;

lazy_static! {
    static ref ZONE_CACHE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

pub struct ZoneManager {}

impl ZoneManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_zone_id(
        &self,
        domain: &str,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<Option<String>, String> {
        if let Some(zone_id) = Self::get_from_cache(domain) {
            return Ok(Some(zone_id));
        }
        self.refresh_zones(email, api_key, api_token).await?;
        Ok(Self::get_from_cache(domain))
    }

    fn get_from_cache(domain: &str) -> Option<String> {
        ZONE_CACHE.read().ok()?.get(domain).cloned()
    }

    async fn refresh_zones(
        &self,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<(), String> {
        let credentials = AuthManager::get_credentials(email, api_key, api_token)?;
        
        // Corrected client instantiation
        let client = HttpApiClientAsync::new(
            credentials,
            Default::default(),
            Environment::Production,
        ).map_err(|e| format!("Cloudflare API client creation failed: {:?}", e))?;
        
        let endpoint = ListZones {
            params: ListZonesParams::default(),
        };
        
        println!("# Retrieving zones from Cloudflare API...");
        
        // Make the async request using the correct client & endpoints
        match client.request(&endpoint).await {
            Ok(api_success) => {
                let zones = api_success.result;
                let mut cache = ZONE_CACHE.write().map_err(|e| format!("Cache write lock error: {}", e))?;
                cache.clear();
                for zone in &zones {
                    println!("# Zone found: {} ({})", zone.name, zone.id);
                    cache.insert(zone.name.clone(), zone.id.clone());
                }
                Ok(())
            }
            Err(ApiFailure::Error(status, errors)) => {
                println!("# Cloudflare API returned error (HTTP {}): {:?}", status, errors);
                Err(format!("Cloudflare API error ({}): {:?}", status, errors))
            }
            Err(ApiFailure::Invalid(e)) => {
                println!("# HTTP request failure: {:?}", e);
                Err(format!("Cloudflare HTTP request failed: {}", e))
            }
        }
    }
}