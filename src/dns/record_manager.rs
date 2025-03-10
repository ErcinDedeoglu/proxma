use anyhow::Result;
use cloudflare::{
    endpoints::dns::{DnsRecord, ListDnsRecords, ListDnsRecordsParams, DnsContent},
    framework::{
        async_api::Client,
        auth::Credentials,
        Environment,
        HttpApiClientConfig,
    },
};

pub struct RecordManager;

impl RecordManager {
    pub fn new() -> Self {
        RecordManager {}
    }
    
    pub async fn get_dns_record(
        zone_id: &str,
        name: &str,
        record_type: &str,
        api_token: Option<&str>,
        api_key: Option<&str>,
        email: Option<&str>,
    ) -> Result<Option<DnsRecord>> {
        let credentials = if let Some(token) = api_token {
            Credentials::UserAuthToken { token: token.to_string() }
        } else if let (Some(key), Some(email)) = (api_key, email) {
            Credentials::UserAuthKey { 
                key: key.to_string(),
                email: email.parse()?,
            }
        } else {
            return Err(anyhow::anyhow!("No valid authentication credentials provided"));
        };

        let client = Client::new(
            credentials,
            HttpApiClientConfig::default(),
            Environment::Production,
        )?;

        let params: ListDnsRecordsParams = ListDnsRecordsParams {
            name: Some(name.to_string()),
            record_type: None,
            ..Default::default()
        };

        let response = client
            .request(&ListDnsRecords {
                zone_identifier: zone_id,
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