// auth_manager.rs
use cloudflare::framework::auth::Credentials;

pub struct AuthManager;

impl AuthManager {
    pub fn get_credentials(
        email: Option<String>, 
        api_key: Option<String>, 
        api_token: Option<String>
    ) -> Result<Credentials, String> {
        if email.is_some() && api_key.is_some() && 
           !email.as_ref().unwrap().is_empty() && 
           !api_key.as_ref().unwrap().is_empty() {
            println!("# Using email & api_key for authentication");
            println!("# email: {}, api_key: {}", 
                &email.as_ref().unwrap().trim().to_string()[..3], 
                &api_key.as_ref().unwrap().trim().to_string()[..3]
            );
            Ok(Credentials::UserAuthKey { 
                email: email.unwrap().trim().to_string(), 
                key: api_key.unwrap().trim().to_string()
            })
        }
        else if api_token.is_some() && !api_token.as_ref().unwrap().is_empty() {
            println!("# Using api_token for authentication");
            println!("# api_token: {}", 
                &api_token.as_ref().unwrap().trim().to_string()[..3]
            );
            Ok(Credentials::UserAuthToken { 
                token: api_token.unwrap().trim().to_string()
            })
        }
        else {
            println!("# No valid credentials provided: must provide either email & api_key, or api_token");
            Err("No valid credentials provided: must provide either email & api_key, or api_token".to_string())
        }
    }
}