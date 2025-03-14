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
            Err(err) => return Err(anyhow::anyhow!(err)),
        };
        
        let client = Client::new(
            credentials,
            ClientConfig::default(),
            Environment::Production,
        )?;
        
        let params: ListDnsRecordsParams = ListDnsRecordsParams {
            name: Some(name.to_string()),
            record_type: None,
            ..Default::default()
        };
        
        let response = client
            .request(&ListDnsRecords {
                zone_identifier: &zone_id,
                params,
            })
            .await?;
            
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
            Err(err) => return Err(anyhow::anyhow!(err)),
        };
        
        let client = Client::new(
            credentials,
            ClientConfig::default(),
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