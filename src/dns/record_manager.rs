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
                _ => false,
            }))
    }
}