use std::process::Command;
use std::io;
use crate::certbot::CertificateRequestResult;

pub fn check_acme_challenge<S: AsRef<str>>(domain: S) -> io::Result<CertificateRequestResult> {
    let domain_str = domain.as_ref();
    println!("Starting ACME challenge for domain: {}", domain_str);
    
    // Use curl with external DNS for all domains
    let challenge_url = format!("http://{}/.well-known/acme-challenge/proxma.proxma", domain_str);
    println!("Checking URL: {}", challenge_url);
    
    // Execute curl command with external DNS
    let curl_output = match Command::new("curl")
        .args([
            "--silent",                             // No progress output
            "--max-time", "10",                     // 10 second timeout
            "--dns-servers", "1.1.1.1,8.8.8.8",     // Use external DNS servers
            &challenge_url
        ])
        .output() {
            Ok(output) => output,
            Err(e) => return Ok(CertificateRequestResult::AcmeChallengeFailure(
                format!("Failed to execute curl command: {}", e)
            )),
        };
        
    // Check if the command was successful
    if !curl_output.status.success() {
        let stderr = String::from_utf8_lossy(&curl_output.stderr);
        return Ok(CertificateRequestResult::AcmeChallengeFailure(
            format!("curl command failed for {}: {}", challenge_url, stderr)
        ));
    }
    
    // Verify the content
    let content = String::from_utf8_lossy(&curl_output.stdout).trim().to_string();
    
    if content != "proxma" {
        return Ok(CertificateRequestResult::AcmeChallengeFailure(
            format!("Incorrect content at {}: Expected 'proxma' but got '{}'", challenge_url, content)
        ));
    }
    
    // Success!
    Ok(CertificateRequestResult::Success)
}