use crate::dns::ZoneManager;
use super::record_manager::RecordManager;
use lazy_static::lazy_static;
use cloudflare::endpoints::dns::dns::DnsContent;

pub struct CloudflareManager;

lazy_static! {
    pub static ref ZONE_MANAGER: ZoneManager = ZoneManager::new();
    pub static ref RECORD_MANAGER: RecordManager = RecordManager::new();
}

impl CloudflareManager {
    pub fn new() -> Self {        
        CloudflareManager
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
        println!("# CLOUDFLARE_MANAGER.add_update_record");
        println!(
            "🔒 Adding DNS record for '{}' with target '{}', type '{}', provider '{}', proxied: {}",
            record, target, r#type, provider, proxied
        );
        
        let zone_id = match ZONE_MANAGER.get_zone_id(&record, email.clone(), api_key.clone(), api_token.clone()).await {
            Ok(Some(id)) => {
                println!("Found zone ID: {}", id);
                id
            },
            Ok(None) => {
                eprintln!("Zone not found");
                return false;
            },
            Err(e) => {
                println!("Error: {}", e);
                return false;
            },
        };
        
        let record_clone = record.clone();
        let dns_record = RecordManager::get_dns_record(
            zone_id, 
            record_clone, 
            r#type, 
            email,
            api_key,
            api_token
        ).await;
        
        let dns_record = dns_record.unwrap();
        
        if dns_record.is_some() {
            println!("Record exists...");
            
            if dns_record.as_ref().unwrap().proxied == proxied { 
                println!("Proxied status is up to date");
                let content_str: String = match &dns_record.as_ref().unwrap().content {
                    DnsContent::A { content } => content.to_string(),
                    DnsContent::AAAA { content } => content.to_string(),
                    DnsContent::CNAME { content } => content.clone(),
                    DnsContent::TXT { content } => content.clone(),
                    _ => String::new(),
                };
                
                if content_str == target {
                    println!("Content is up to date");
                    
                    if dns_record.as_ref().unwrap().name == record {
                        println!("Name is up to date");
                        return true;
                        
                    }
                }              
            }
        }
        return true;
    }
}