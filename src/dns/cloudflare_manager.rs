use lazy_static::lazy_static;
use crate::dns::ZoneManager;
use super::record_manager::RecordManager;

use cloudflare::{
    endpoints::dns::{CreateDnsRecord, CreateDnsRecordParams, DnsContent},
    framework::{auth::Credentials, Environment, HttpApiClientConfig, async_api::Client},
};

pub struct CloudflareManager {
    client: Client,
}

lazy_static! {
    pub static ref ZONE_MANAGER: ZoneManager = ZoneManager::new();
    pub static ref RECORD_MANAGER: RecordManager = RecordManager::new();
}

impl CloudflareManager {
    pub fn new() -> Self {
        let credentials: Credentials = Credentials::UserAuthToken { 
            token: String::new()
        };
        
        let client = Client::new(
            credentials,
            HttpApiClientConfig::default(),
            Environment::Production,
        ).expect("Failed to create default client");

        CloudflareManager { client }
    }

    pub async fn add_update_record(
        &self,
        provider: String,
        record: String,
        target: String,
        r#type: String,
        proxied: bool,
        email: String,
        api_key: String,
        api_token: String,
    ) -> bool {
        println!("# CLOUDFLARE_MANAGER.add_update_record");
        println!(
            "🔒 Adding DNS record for '{}' with target '{}', type '{}', provider '{}', proxied: {}",
            record, target, r#type, provider, proxied
        );

        let zone_id = match ZONE_MANAGER.get_zone_id(&record, Some(email.clone()), Some(api_key.clone()), Some(api_token.clone())).await {
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

        let dns_record = RecordManager::get_dns_record(&zone_id, &record, &r#type, Some(&api_token), Some(&api_key), Some(&email)).await;

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



        // let credentials = if !api_token.is_empty() {
        //     Credentials::UserAuthToken { token: api_token }
        // } else {
        //     Credentials::UserAuthKey { email, key: api_key }
        // };

        // let client = match Client::new(
        //     credentials,
        //     HttpApiClientConfig::default(),
        //     Environment::Production,
        // ) {
        //     Ok(client) => client,
        //     Err(e) => {
        //         eprintln!("Failed to create client: {:?}", e);
        //         return false;
        //     }
        // };

        // // Create DNS content based on record type
        // let dns_content = match r#type.to_uppercase().as_str() {
        //     "A" => match target.parse() {
        //         Ok(ip) => DnsContent::A { content: ip },
        //         Err(_) => {
        //             eprintln!("Invalid IP address");
        //             return false;
        //         }
        //     },
        //     "AAAA" => match target.parse() {
        //         Ok(ip) => DnsContent::AAAA { content: ip },
        //         Err(_) => {
        //             eprintln!("Invalid IPv6 address");
        //             return false;
        //         }
        //     },
        //     "CNAME" => DnsContent::CNAME { content: target },
        //     "TXT" => DnsContent::TXT { content: target },
        //     _ => {
        //         eprintln!("Unsupported record type: {}", r#type);
        //         return false;
        //     }
        // };

        // let create_request = CreateDnsRecord {
        //     zone_identifier: &provider,
        //     params: CreateDnsRecordParams {
        //         name: &record,
        //         content: dns_content,
        //         ttl: None,
        //         priority: None,
        //         proxied: Some(proxied),
        //     },
        // };

        // match client.request(&create_request).await {
        //     Ok(_) => {
        //         println!("Successfully created record");
        //         true
        //     }
        //     Err(e) => {
        //         eprintln!("Failed to create record: {:?}", e);
        //         false
        //     }
        // }
    }
}