use anyhow::Result;
use super::auth_manager::AuthManager;
use cloudflare::{
    endpoints::dns::dns::{DnsContent, DnsRecord, ListDnsRecords, ListDnsRecordsParams},
    framework::{client::{
        async_api::Client, ClientConfig,
    }, Environment},
};

pub struct RecordManager;

impl RecordManager {
    pub fn new() -> Self {
        RecordManager {}
    }
    
    /// Test network connectivity to Cloudflare API using existing credentials
    pub async fn test_connectivity(
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<()> {
        println!("🔍 Testing network connectivity to Cloudflare API...");
        
        // First test DNS resolution
        println!("   - Testing DNS resolution for api.cloudflare.com...");
        match tokio::net::lookup_host("api.cloudflare.com:443").await {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    println!("   ✅ DNS resolution successful: {}", addr.ip());
                } else {
                    println!("   ❌ DNS resolution returned no addresses");
                }
            },
            Err(e) => {
                eprintln!("   ❌ DNS resolution failed: {}", e);
                return Err(anyhow::anyhow!("DNS resolution failed: {}", e));
            }
        }
        
        let credentials = match AuthManager::get_credentials(email, api_key, api_token) {
            Ok(it) => it,
            Err(err) => return Err(anyhow::anyhow!("Authentication failed for connectivity test: {}", err)),
        };
        
        let mut config = ClientConfig::default();
        config.http_timeout = std::time::Duration::from_secs(10);
        
        let client = Client::new(
            credentials,
            config,
            Environment::Production,
        )?;
        
        // Try to verify token/credentials by listing zones (simpler endpoint)
        println!("   - Testing HTTPS connection and authentication...");
        
        // Use the same zone listing approach as zone_manager.rs
        use cloudflare::endpoints::zones::zone::{ListZones, ListZonesParams};
        
        let params = ListZonesParams {
            page: Some(1),
            per_page: Some(1), // Just get one zone to test connectivity
            ..Default::default()
        };
        
        match client.request(&ListZones { params }).await {
            Ok(_) => {
                println!("✅ Network connectivity and authentication to Cloudflare API: OK");
                Ok(())
            },
            Err(e) => {
                eprintln!("❌ Cloudflare API connectivity/authentication test failed: {}", e);
                Err(anyhow::anyhow!("Cloudflare API test failed: {}", e))
            }
        }
    }
    
    pub async fn get_dns_record(
        zone_id: String,
        name: String,
        record_type: String,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<Option<DnsRecord>> {
        let credentials = match AuthManager::get_credentials(email, api_key, api_token) {
            Ok(it) => it,
            Err(err) => return Err(anyhow::anyhow!("Authentication failed: {}", err)),
        };
        
        let mut config = ClientConfig::default();
        config.http_timeout = std::time::Duration::from_secs(30);
        
        // Add more robust HTTP client configuration
        println!("🌐 Configuring HTTP client for Cloudflare API request");
        println!("   - Timeout: 30 seconds");
        println!("   - Target URL: https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}", zone_id, name);
        println!("   - Record Type Filter: {}", record_type);
        
        // Check if we're running in a container environment
        if std::path::Path::new("/.dockerenv").exists() {
            println!("   - Running in Docker container - checking network connectivity");
        }
        
        let client = Client::new(
            credentials,
            config,
            Environment::Production,
        )?;
        
        let params: ListDnsRecordsParams = ListDnsRecordsParams {
            name: Some(name.to_string()),
            record_type: None,
            ..Default::default()
        };
        
        let response = match client
            .request(&ListDnsRecords {
                zone_identifier: &zone_id,
                params,
            })
            .await {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("❌ HTTP request failed for DNS record lookup:");
                    eprintln!("   - Zone ID: {}", zone_id);
                    eprintln!("   - Record Name: {}", name);
                    eprintln!("   - URL: https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}", zone_id, name);
                    eprintln!("   - Error: {}", e);
                    eprintln!("   - Possible causes:");
                    eprintln!("     * Network connectivity issues");
                    eprintln!("     * DNS resolution problems");
                    eprintln!("     * Firewall blocking outbound HTTPS requests");
                    eprintln!("     * Invalid Cloudflare API credentials");
                    eprintln!("     * Cloudflare API service unavailable");
                    return Err(anyhow::anyhow!("HTTP request failed: {}", e));
                }
            };
            
        Ok(response.result
            .into_iter()
            .find(|record| match &record.content {
                DnsContent::A { .. } => record_type == "A",
                DnsContent::AAAA { .. } => record_type == "AAAA",
                DnsContent::CNAME { .. } => record_type == "CNAME",
                DnsContent::MX { .. } => record_type == "MX",
                DnsContent::NS { .. } => record_type == "NS",
                DnsContent::TXT { .. } => record_type == "TXT",
                DnsContent::SRV { .. } => record_type == "SRV",
            }))
    }

    pub async fn add_update_record(
        zone_id: String,
        name: String,
        record_type: String,
        content: String,
        ttl: Option<u32>,
        proxied: Option<bool>,
        existing_record: Option<DnsRecord>,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>,
    ) -> Result<DnsRecord> {
        use cloudflare::endpoints::dns::dns::{CreateDnsRecord, CreateDnsRecordParams, UpdateDnsRecord, UpdateDnsRecordParams};
        use std::net::{Ipv4Addr, Ipv6Addr};
    
        let credentials = match AuthManager::get_credentials(email, api_key, api_token) {
            Ok(it) => it,
            Err(err) => return Err(anyhow::anyhow!("Authentication failed: {}", err)),
        };
        
        let mut config = ClientConfig::default();
        config.http_timeout = std::time::Duration::from_secs(30);
        
        let client = Client::new(
            credentials,
            config,
            Environment::Production,
        )?;
        
        // Create DNS content based on record type
        let dns_content = match record_type.as_str() {
            "A" => DnsContent::A { 
                content: content.parse::<Ipv4Addr>()? 
            },
            "AAAA" => DnsContent::AAAA { 
                content: content.parse::<Ipv6Addr>()? 
            },
            "CNAME" => DnsContent::CNAME { 
                content 
            },
            "TXT" => DnsContent::TXT { 
                content 
            },
            "MX" => {
                // MX records require priority
                let parts: Vec<&str> = content.split(' ').collect();
                if parts.len() != 2 {
                    return Err(anyhow::anyhow!("MX record content should be in format 'priority hostname'"));
                }
                let priority = parts[0].parse::<u16>()?;
                DnsContent::MX {
                    content: parts[1].to_string(),
                    priority,
                }
            },
            "NS" => DnsContent::NS { 
                content 
            },
            "SRV" => DnsContent::SRV { 
                content 
            },
            _ => return Err(anyhow::anyhow!("Unsupported record type: {}", record_type)),
        };
    
        match existing_record {
            Some(record) => {
                // Update existing record
                let endpoint = UpdateDnsRecord {
                    zone_identifier: &zone_id,
                    identifier: &record.id,
                    params: UpdateDnsRecordParams {
                        name: &name,
                        content: dns_content,
                        ttl,
                        proxied,
                    },
                };
                
                let response = client.request(&endpoint).await?;
                Ok(response.result)
            },
            None => {
                // Create new record
                let endpoint = CreateDnsRecord {
                    zone_identifier: &zone_id,
                    params: CreateDnsRecordParams {
                        name: &name,
                        content: dns_content,
                        ttl,
                        priority: None,
                        proxied,
                    },
                };
                
                let response = client.request(&endpoint).await?;
                Ok(response.result)
            }
        }
    }
}