use hmac::{Hmac, KeyInit, Mac as _};
use sha2::Sha256;

pub type Secret = Hmac<Sha256>;

pub fn parse(s: &str) -> Result<Secret, String> {
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    if bytes.len() < 32 {
        return Err("must be at least 32 bytes".into());
    }

    Secret::new_from_slice(&bytes).map_err(|_| "invalid HMAC key".into())
}

pub fn sign(secret: &Secret, url: &str) -> String {
    let mut mac = secret.clone();
    mac.update(url.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify(secret: &Secret, url: &str, sig_hex: &str) -> bool {
    let Ok(sig) = hex::decode(sig_hex) else {
        return false;
    };

    let mut mac = secret.clone();
    mac.update(url.as_bytes());
    mac.verify_slice(&sig).is_ok()
}
