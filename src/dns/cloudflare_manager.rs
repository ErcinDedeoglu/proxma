use crate::dns::ZoneManager;
use crate::dns::DNSManager;
use crate::dns::record_manager::RecordManager;
use lazy_static::lazy_static;


pub struct CloudflareManager;

lazy_static! {
    pub static ref ZONE_MANAGER: ZoneManager = ZoneManager::new();
    pub static ref DNS_MANAGER: DNSManager = DNSManager::new();
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

        // Use find_best_matching_zone instead of get_zone_id
        let (zone_name, zone_id) = match ZONE_MANAGER.find_best_matching_zone(
            &record,
            email.clone(),
            api_key.clone(),
            api_token.clone()
        ).await {
            Ok((Some(name), Some(id))) => {
                println!("Found best matching zone: {} (ID: {})", name, id);
                (name, id)
            },
            Ok(_) => {
                eprintln!("No matching zone found for record '{}'", record);
                return false;
            },
            Err(e) => {
                eprintln!("Error finding matching zone: {}", e);
                return false;
            },
        };

        let record_name = if record == zone_name {
            record.clone()
        } else if record.ends_with(&format!(".{}", zone_name)) {
            record.clone()
        } else {
            format!("{}.{}", record, zone_name)
        };

        let dns_record = RecordManager::get_dns_record(
            zone_id,
            record_name.clone(),
            r#type.clone(),
            email.clone(),
            api_key.clone(),
            api_token.clone()
        ).await;

        let dns_record = match dns_record {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Error getting DNS record: {}", e);
                return false;
            }
        };

        let mut record_update: bool = false;

        if let Some(existing_record) = dns_record {
            println!("Record exists...");

            if existing_record.proxied != proxied {
                println!("Proxied status is out of date");
                record_update = true;
            }
            
            if r#type != DNSManager::dns_content_type_string(&existing_record.content) {
                println!("Record type is out of date");
                record_update = true;
            }
            
            if record != DNSManager::dns_content_to_string(&existing_record.content) {
                println!("Record name is out of date");
                record_update = true;
            }
        } else {
            println!("Creating new record...");
            record_update = true;
        }

        if record_update {
            println!("Adding/Updating record...");
            DNS_MANAGER.add_update_record(
                provider.clone(),  // provider
                record_name,       // record
                target,            // target
                r#type,            // r#type
                proxied,           // proxied (boolean, not Option<bool>)
                email.clone(),     // email
                api_key.clone(),   // api_key
                api_token.clone()  // api_token
            ).await;
        } else {
            println!("Record is already up to date, no update needed");
        }

        return true;
    }
}