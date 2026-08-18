use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// `X-Hub-Signature-256` value (`sha256=<hex>`).
pub fn signature_header(secret: &[u8], body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(body);
    let bytes = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(bytes))
}

pub fn verify_signature(secret: &[u8], body: &[u8], header: &str) -> bool {
    let expected = signature_header(secret, body);
    // Length-mismatch must fail; hmac crate compare via verify_slice on equal length hex.
    constant_eq(expected.as_bytes(), header.trim().as_bytes())
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut x = 0u8;
    for (l, r) in a.iter().zip(b.iter()) {
        x |= l ^ r;
    }
    x == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_style_hmac() {
        let sig = signature_header(b"secret", b"{\"action\":\"opened\"}");
        assert!(verify_signature(b"secret", b"{\"action\":\"opened\"}", &sig));
        assert!(!verify_signature(b"secret", b"{\"action\":\"opened\"}", "sha256=deadbeef"));
        assert!(!verify_signature(b"nope", b"{\"action\":\"opened\"}", &sig));
    }
}
