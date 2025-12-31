//! Express-session compatible cookie signature
//!
//! This module implements cookie signing compatible with Node.js cookie-signature library.
//! The format is: `s:` + session_id + `.` + base64(hmac_sha256(session_id, secret))

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Sign a value using the express-session compatible format.
/// Returns: `s:` + value + `.` + base64_signature (without padding)
///
/// This matches Node.js cookie-signature format:
/// ```javascript
/// exports.sign = function(val, secret){
///   return val + '.' + crypto
///     .createHmac('sha256', secret)
///     .update(val)
///     .digest('base64')
///     .replace(/\=+$/, '');
/// };
/// ```
pub fn sign(value: &str, secret: &str) -> String {
    let signature = create_signature(value, secret);
    format!("s:{}.{}", value, signature)
}

/// Create HMAC-SHA256 signature in base64 format (no padding, to match Node.js)
fn create_signature(value: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(value.as_bytes());
    let result = mac.finalize();
    // Node.js uses standard base64 and strips trailing '=' padding
    STANDARD
        .encode(result.into_bytes())
        .trim_end_matches('=')
        .to_string()
}

/// Unsign a value, verifying the signature.
/// Expects format: `s:` + value + `.` + signature
/// Returns the original value if signature is valid, None otherwise.
///
/// This matches Node.js cookie-signature format:
/// ```javascript
/// exports.unsign = function(input, secret){
///   var tentativeValue = input.slice(0, input.lastIndexOf('.')),
///       expectedInput = exports.sign(tentativeValue, secret);
///   return sha(input) == sha(expectedInput) ? tentativeValue : false;
/// };
/// ```
pub fn unsign(signed_value: &str, secret: &str) -> Option<String> {
    // Check for 's:' prefix
    if !signed_value.starts_with("s:") {
        return None;
    }

    let without_prefix = &signed_value[2..];

    // Find the last '.' which separates value from signature
    let dot_pos = without_prefix.rfind('.')?;
    let value = &without_prefix[..dot_pos];
    let provided_signature = &without_prefix[dot_pos + 1..];

    // Create expected signature
    let expected_signature = create_signature(value, secret);

    // Constant-time comparison to prevent timing attacks
    if constant_time_compare(&expected_signature, provided_signature) {
        Some(value.to_string())
    } else {
        None
    }
}

/// Try to unsign with multiple secrets (for secret rotation)
pub fn unsign_with_secrets(signed_value: &str, secrets: &[String]) -> Option<String> {
    for secret in secrets {
        if let Some(value) = unsign(signed_value, secret) {
            return Some(value);
        }
    }
    None
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_unsign() {
        let secret = "keyboard cat";
        let value = "test-session-id";

        let signed = sign(value, secret);
        assert!(signed.starts_with("s:"));

        let unsigned = unsign(&signed, secret);
        assert_eq!(unsigned, Some(value.to_string()));
    }

    #[test]
    fn test_invalid_signature() {
        let secret = "keyboard cat";
        let value = "test-session-id";

        let signed = sign(value, secret);
        let unsigned = unsign(&signed, "wrong secret");
        assert_eq!(unsigned, None);
    }

    #[test]
    fn test_no_prefix() {
        let unsigned = unsign("test-session-id.signature", "secret");
        assert_eq!(unsigned, None);
    }

    #[test]
    fn test_compatible_with_express() {
        // Test vector from express-session / cookie-signature
        // These values can be verified with Node.js:
        // const signature = require('cookie-signature');
        // console.log(signature.sign('my session id', 'secret'));
        // Result: 'my session id.Jytwl6nuMV42lj6Ldd7aa4sboVs87ZnnCfYLCAm7OrU'

        let secret = "secret";
        let value = "my session id";
        let signed = sign(value, secret);

        // The signed value should be exactly what Node.js produces (with s: prefix)
        assert_eq!(
            signed,
            "s:my session id.Jytwl6nuMV42lj6Ldd7aa4sboVs87ZnnCfYLCAm7OrU"
        );

        // Verify we can unsign our own signature
        let unsigned = unsign(&signed, secret);
        assert_eq!(unsigned, Some(value.to_string()));
    }

    #[test]
    fn test_secret_rotation() {
        let old_secret = "old-secret".to_string();
        let new_secret = "new-secret".to_string();
        let value = "session-id";

        // Sign with old secret
        let signed = sign(value, &old_secret);

        // Should work with both secrets in the list
        let secrets = vec![new_secret, old_secret];
        let unsigned = unsign_with_secrets(&signed, &secrets);
        assert_eq!(unsigned, Some(value.to_string()));
    }
}
