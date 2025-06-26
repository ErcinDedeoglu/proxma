use cloudflare::framework::auth::Credentials;

pub struct AuthManager;

impl AuthManager {
    pub fn get_credentials(
        email: Option<String>, 
        api_key: Option<String>, 
        api_token: Option<String>
    ) -> Result<Credentials, String> {
        // Debug credential availability
        let email_provided = email.as_ref().map_or(false, |e| !e.is_empty());
        let api_key_provided = api_key.as_ref().map_or(false, |k| !k.is_empty());
        let api_token_provided = api_token.as_ref().map_or(false, |t| !t.is_empty());
        
        println!("# Credential check - email: {}, api_key: {}, api_token: {}",
                 email_provided, api_key_provided, api_token_provided);
        
        if email_provided && api_key_provided {
            println!("# Using email & api_key for authentication");
            let email_str = email.as_ref().unwrap().trim();
            let api_key_str = api_key.as_ref().unwrap().trim();
            
            if email_str.len() >= 3 && api_key_str.len() >= 3 {
                println!("# email: {}..., api_key: {}...",
                    &email_str[..3],
                    &api_key_str[..3]
                );
            }
            
            Ok(Credentials::UserAuthKey {
                email: email_str.to_string(),
                key: api_key_str.to_string()
            })
        }
        else if api_token_provided {
            println!("# Using api_token for authentication");
            let token_str = api_token.as_ref().unwrap().trim();
            
            if token_str.len() >= 3 {
                println!("# api_token: {}...", &token_str[..3]);
            }
            
            Ok(Credentials::UserAuthToken {
                token: token_str.to_string()
            })
        }
        else {
            let error_msg = format!(
                "No valid credentials provided. Available: email={}, api_key={}, api_token={}. Must provide either (email AND api_key) OR api_token",
                email_provided, api_key_provided, api_token_provided
            );
            println!("# {}", error_msg);
            Err(error_msg)
        }
    }
}