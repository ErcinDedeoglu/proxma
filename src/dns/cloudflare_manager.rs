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

        // Create the proper record name relative to the zone
        // If the record is exactly the zone or ends with the zone, we need to adjust
        let record_name = if record == zone_name {
            // For apex record (exactly the zone), use the record as is
            record.clone()
        } else if record.ends_with(&format!(".{}", zone_name)) {
            // For subdomains, use the record as is
            record.clone()
        } else {
            // For other cases, we might need to append the zone
            // This is a fallback case and might need adjustment based on your needs
            format!("{}.{}", record, zone_name)
        };

        let dns_record = RecordManager::get_dns_record(
            zone_id,
            record_name.clone(),
            r#type,
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

        if let Some(existing_record) = dns_record {
            println!("Record exists...");
            if existing_record.proxied == proxied {
                println!("Proxied status is up to date");
                let content_str: String = match &existing_record.content {
                    DnsContent::A { content } => content.to_string(),
                    DnsContent::AAAA { content } => content.to_string(),
                    DnsContent::CNAME { content } => content.clone(),
                    DnsContent::TXT { content } => content.clone(),
                    _ => String::new(),
                };
                if content_str == target {
                    println!("Content is up to date");
                    if existing_record.name == record_name {
                        println!("Name is up to date");
                        return true;
                    }
                }
            }

            // Here you would add code to update the existing record
            // This part seems to be missing from your original code
            println!("Updating existing record...");
            // RECORD_MANAGER.update_record(...) implementation would go here
        } else {
            // Here you would add code to create a new record
            // This part seems to be missing from your original code
            println!("Creating new record...");
            // RECORD_MANAGER.create_record(...) implementation would go here
        }

        return true;
    }
}