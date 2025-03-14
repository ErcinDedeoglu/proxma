use cloudflare::endpoints::dns::dns::DnsContent;
use lazy_static::lazy_static;

use super::{record_manager::RecordManager, CloudflareManager};

pub struct DNSManager {
}

lazy_static! {
    pub static ref CLOUDFLARE_MANAGER: CloudflareManager = CloudflareManager::new();
    pub static ref RECORD_MANAGER: RecordManager = RecordManager::new();
}

impl DNSManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn dns_content_to_string(content: &DnsContent) -> String {
        match content {
            DnsContent::A { content: ip } => ip.to_string(),
            DnsContent::AAAA { content: ip } => ip.to_string(),
            DnsContent::CNAME { content: cname } => cname.clone(),
            DnsContent::NS { content: ns } => ns.clone(),
            DnsContent::MX { content: mx, priority } => format!("{} {}", priority, mx),
            DnsContent::TXT { content: txt } => txt.clone(),
            DnsContent::SRV { content: srv } => srv.clone(),
        }
    }

    pub fn dns_content_type_string(content: &DnsContent) -> String {
        match content {
            DnsContent::A { .. } => "A".to_string(),
            DnsContent::AAAA { .. } => "AAAA".to_string(),
            DnsContent::CNAME { .. } => "CNAME".to_string(),
            DnsContent::NS { .. } => "NS".to_string(),
            DnsContent::MX { .. } => "MX".to_string(),
            DnsContent::TXT { .. } => "TXT".to_string(),
            DnsContent::SRV { .. } => "SRV".to_string(),
        }
    }
    
    pub async fn add_update_record(
        &self, 
        provider: String, 
        record: String, 
        target: String, 
        r#type: String, 
        proxied: bool,
        email: Option<String>,
        api_key: Option<String>,
        api_token: Option<String>
    ) -> bool {
        println!("🔒 Adding DNS record for '{}' with target '{}', type '{}', provider '{}', proxied: {}", record, target, r#type, provider, proxied);
                 
        if provider == "cloudflare" {
            Box::pin(CLOUDFLARE_MANAGER.add_update_record(
                provider, 
                record, 
                target, 
                r#type, 
                proxied,
                email,
                api_key,
                api_token
            )).await
        } else {
            false
        }
    }
}