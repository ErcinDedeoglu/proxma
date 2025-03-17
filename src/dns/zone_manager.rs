use cloudflare::framework::client::async_api::Client as HttpApiClientAsync;
use cloudflare::framework::{Environment, response::ApiFailure};
use cloudflare::endpoints::zones::zone::{ListZones, ListZonesParams};
use lazy_static::lazy_static;
use tokio::sync::RwLockWriteGuard;
use std::collections::HashMap;
use tokio::sync::RwLock;
use super::auth_manager::AuthManager;
use std::sync::Arc;

lazy_static! {
    static ref ZONE_CACHE: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct ZoneManager {}

impl ZoneManager {
    pub fn new() -> Self {
        Self {}
    }

    async fn refresh_zones(
        &self,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<(), String> {
        let credentials = AuthManager::get_credentials(email, api_key, api_token)?;
    
        let client = HttpApiClientAsync::new(
            credentials,
            Default::default(),
            Environment::Production,
        )
        .map_err(|e| format!("Cloudflare API client creation failed: {:?}", e))?;
    
        let mut page = 1;
        let per_page = 50;
        let mut fetched_all = false;
    
        {
            let mut cache: RwLockWriteGuard<'_, HashMap<String, String>> = ZONE_CACHE.write().await;
            cache.clear();
        }
    
        println!("# Retrieving zones from Cloudflare API...");
        while !fetched_all {
            let params = ListZonesParams {
                page: Some(page),
                per_page: Some(per_page),
                ..Default::default()
            };
            let endpoint = ListZones { params };
    
            match client.request(&endpoint).await {
                Ok(api_success) => {
                    let zones = api_success.result;
    
                    {
                        let mut cache = ZONE_CACHE.write().await;
                        for zone in &zones {
                            println!("# Zone found: {} ({})", zone.name, zone.id);
                            cache.insert(zone.name.clone(), zone.id.clone());
                        }
                    }
    
                    if let Some(result_info) = api_success.result_info {
                        if let Some(total_pages) = result_info.get("total_pages").and_then(|v| v.as_u64()) {
                            if page >= total_pages as u32 {
                                fetched_all = true;
                            } else {
                                page += 1;
                            }
                        } else {
                            fetched_all = true;
                        }
                    } else {
                        fetched_all = true;
                    }
                }
                Err(ApiFailure::Error(status, errors)) => {
                    println!("# Cloudflare API returned error (HTTP {}): {:?}", status, errors);
                    return Err(format!("Cloudflare API error ({}): {:?}", status, errors));
                }
                Err(ApiFailure::Invalid(e)) => {
                    println!("# HTTP request failure: {:?}", e);
                    return Err(format!("Cloudflare HTTP request failed: {}", e));
                }
            }
        }
        Ok(())
    }

    pub async fn find_best_matching_zone(
        &self,
        hostname: &str,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<(Option<String>, Option<String>), String> {
        // Try to find from cache first
        if let Some((zone_name, zone_id)) = Self::find_best_match_from_cache(hostname).await {
            return Ok((Some(zone_name), Some(zone_id)));
        }

        // If not found in cache, refresh zones and try again
        self.refresh_zones(email, api_key, api_token).await?;

        // Return the best match after refresh
        Ok(Self::find_best_match_from_cache(hostname).await.map(|(name, id)| (Some(name), Some(id))).unwrap_or((None, None)))
    }

    /// Helper method to find the best matching zone from the cache
    async fn find_best_match_from_cache(hostname: &str) -> Option<(String, String)> {
        let cache = ZONE_CACHE.read().await;

        // Find all zone names that are part of the hostname (domain matching)
        // A proper match is when the hostname ends with the zone name,
        // or when the hostname equals the zone name, or when the hostname
        // contains the zone name followed by a dot
        let matching_zones: Vec<(String, String)> = cache.iter()
            .filter(|(zone_name, _)| {
                hostname == *zone_name ||
                    hostname.ends_with(&format!(".{}", zone_name)) ||
                    hostname.contains(&format!("{}.{}", zone_name, ""))
            })
            .map(|(zone_name, zone_id)| (zone_name.clone(), zone_id.clone()))
            .collect();

        if matching_zones.is_empty() {
            return None;
        }

        // Find the longest matching zone name (best match)
        matching_zones
            .into_iter()
            .max_by_key(|(zone_name, _)| zone_name.len())
    }
}