use cloudflare::{
    endpoints::zone::{ListZones, ListZonesParams},
    framework::{
        async_api::Client,
        auth::Credentials,
        Environment,
        HttpApiClientConfig,
        response::ApiResponse,
    },
};
use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    static ref ZONE_CACHE: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

pub struct ZoneManager {}

impl ZoneManager {
    pub fn new() -> Self {
        ZoneManager {}
    }

    pub async fn get_zone_id(&self, domain: &str, email: Option<String>, api_key: Option<String>, api_token: Option<String>) -> Result<Option<String>, String> {
        // First, try to get from cache
        if let Some(zone_id) = Self::get_from_cache(domain) {
            return Ok(Some(zone_id));
        }

        // If not in cache, refresh zones and try again
        self.refresh_zones(email, api_key, api_token).await?;
        
        Ok(Self::get_from_cache(domain))
    }

    // Private helper functions
    fn get_from_cache(domain: &str) -> Option<String> {
        ZONE_CACHE.read()
            .ok()?
            .get(domain)
            .cloned()
    }

    async fn refresh_zones(&self, email: Option<String>, api_key: Option<String>, api_token: Option<String>) -> Result<(), String> {
        let credentials = match (email, api_key, api_token) {
            (Some(email), Some(api_key), _) if !email.is_empty() && !api_key.is_empty() => {
                Credentials::UserAuthKey {
                    email,
                    key: api_key,
                }
            }
            (_, _, Some(token)) if !token.is_empty() => {
                Credentials::UserAuthToken { token }
            }
            _ => return Err("No valid credentials provided (empty or missing)".to_string()),
        };
    
        let client = match Client::new(credentials, HttpApiClientConfig::default(), Environment::Production) {
            Ok(client) => client,
            Err(e) => return Err(format!("Failed to create client: {}", e)),
        };
    
        match client.request(&ListZones { params: ListZonesParams::default() }).await {
            Ok(response) => {
                let mut cache = ZONE_CACHE.write().map_err(|e| e.to_string())?;
                cache.clear();
                for zone in response.result {
                    cache.insert(zone.name, zone.id);
                }
                Ok(())
            },
            Err(e) => Err(format!("Failed to list zones: {}", e))
        }
    }
}