use std::sync::OnceLock;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

const DOMAIN_SALT: &[u8] = b"AnyContext::HexagonalVectorEncryption::v1";
const INTERNAL_PEPPER: &[u8] = b"actx_sec_pepper_8f4a29c17e0b5d3a91";

/// Hardware-bound encryption & decryption engine for AnyContext vector storage.
/// Guarantees that vector chunk payloads (text, summary, keywords) encrypted
/// with AES-GCM-256 by Python or Rust are seamlessly decrypted in memory.
pub struct NativeSecurityEngine {
    machine_id: String,
    key: [u8; 32],
}

impl NativeSecurityEngine {
    /// Creates a new security engine, optionally overriding the machine identifier.
    pub fn new(machine_id_override: Option<&str>) -> Self {
        let machine_id = match machine_id_override {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => Self::extract_machine_identifier(),
        };
        let key = Self::derive_aesgcm_key(&machine_id);
        Self { machine_id, key }
    }

    /// Accessor for global singleton instance.
    pub fn get_instance() -> &'static Self {
        static INSTANCE: OnceLock<NativeSecurityEngine> = OnceLock::new();
        INSTANCE.get_or_init(|| NativeSecurityEngine::new(None))
    }

    /// Returns the active machine identifier.
    pub fn machine_id(&self) -> &str {
        &self.machine_id
    }

    /// Derives a 256-bit AES-GCM key using PBKDF2-HMAC-SHA256 with 100,000 rounds.
    fn derive_aesgcm_key(machine_id: &str) -> [u8; 32] {
        let pepper_str = std::str::from_utf8(INTERNAL_PEPPER).unwrap_or("actx_sec_pepper_8f4a29c17e0b5d3a91");
        let combined_secret = format!("{}::{}", machine_id, pepper_str);
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(combined_secret.as_bytes(), DOMAIN_SALT, 100_000, &mut key);
        key
    }

    /// Extracts a unique hardware/operating-system identifier.
    /// Supports Windows (MachineGuid / CSPProduct UUID), Linux (machine-id), macOS (IOPlatformUUID).
    pub fn extract_machine_identifier() -> String {
        // 1. Environment variable override
        if let Ok(env_id) = std::env::var("ACTX_MACHINE_ID") {
            let trimmed = env_id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        // 2. Linux-specific machine-id retrieval
        #[cfg(target_os = "linux")]
        {
            for mid_path in &["/etc/machine-id", "/var/lib/dbus/machine-id"] {
                if let Ok(content) = std::fs::read_to_string(mid_path) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return format!("lin_mid_{}", trimmed);
                    }
                }
            }
        }

        // 3. Windows-specific UUID retrieval
        #[cfg(target_os = "windows")]
        {
            // Try reg query for MachineGuid
            let output = std::process::Command::new("reg")
                .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
                .output();
            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if line.contains("MachineGuid") && line.contains("REG_SZ") {
                        if let Some(guid) = line.split("REG_SZ").nth(1) {
                            let clean = guid.trim();
                            if clean.len() > 10 {
                                return format!("win_guid_{}", clean);
                            }
                        }
                    }
                }
            }

            // Fallback: wmic csproduct get uuid
            let wmic_out = std::process::Command::new("wmic")
                .args(["csproduct", "get", "uuid"])
                .output();
            if let Ok(out) = wmic_out {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let clean = line.trim();
                    if !clean.is_empty() && !clean.to_lowercase().contains("uuid") && clean.len() > 10 {
                        return format!("win_csp_{}", clean);
                    }
                }
            }
        }

        // 4. macOS-specific UUID retrieval
        #[cfg(target_os = "macos")]
        {
            let ioreg_out = std::process::Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output();
            if let Ok(out) = ioreg_out {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if line.contains("IOPlatformUUID") {
                        if let Some(raw_uuid) = line.split('=').nth(1) {
                            let clean = raw_uuid.trim().trim_matches('"');
                            if !clean.is_empty() {
                                return format!("mac_uuid_{}", clean);
                            }
                        }
                    }
                }
            }
        }

        // 5. Robust Fallback across all platforms
        let host = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "localhost".to_string());
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let combined = format!("{}::{}::{}", host, os, arch);
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        format!("fallback_{:x}", hasher.finalize())
    }

    /// Decrypts an 'enc::' prefixed ciphertext string using AES-GCM-256.
    /// Returns the original plaintext if not encrypted (retrocompatible).
    pub fn decrypt_text(&self, cipher_or_plain: &str) -> String {
        if cipher_or_plain.is_empty() {
            return String::new();
        }
        if !cipher_or_plain.starts_with("enc::") {
            return cipher_or_plain.to_string();
        }

        let raw_b64 = &cipher_or_plain[5..];
        let payload = match BASE64.decode(raw_b64) {
            Ok(p) => p,
            Err(_) => return cipher_or_plain.to_string(),
        };

        // Standard payload: 12-byte nonce + at least 16-byte authentication tag
        if payload.len() < 28 {
            return cipher_or_plain.to_string();
        }

        let nonce_bytes = &payload[..12];
        let ciphertext = &payload[12..];

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce_bytes);

        match cipher.decrypt(nonce, ciphertext) {
            Ok(decrypted_bytes) => {
                String::from_utf8(decrypted_bytes)
                    .unwrap_or_else(|_| "[Corrupted Decrypted UTF-8 Content]".to_string())
            }
            Err(_) => {
                "[Protected Context Data - Hardware Key Mismatch]".to_string()
            }
        }
    }

    /// Encrypts a plaintext string using AES-GCM-256.
    /// Returns a base64 encoded string prefixed with 'enc::'.
    pub fn encrypt_text(&self, plaintext: &str) -> String {
        if plaintext.is_empty() {
            return String::new();
        }
        if plaintext.starts_with("enc::") {
            return plaintext.to_string();
        }

        use aes_gcm::aead::OsRng;
        use aes_gcm::aead::rand_core::RngCore;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        match cipher.encrypt(nonce, plaintext.as_bytes()) {
            Ok(ciphertext) => {
                let mut payload = nonce_bytes.to_vec();
                payload.extend_from_slice(&ciphertext);
                format!("enc::{}", BASE64.encode(&payload))
            }
            Err(_) => plaintext.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_security_engine_interop_decryption() {
        // Test vector produced directly by Python's SecurityEngine with machine_id='test_machine_123'
        let python_ciphertext = "enc::8DvCwkmHP0BT74xMJRySDrFjqQUOrfelh1C6WAH6Z+aGhmJH0G9OECmsLCuBhptCWlEx1ySIjw==";
        let expected_plaintext = "Hello AnyContext Decryption";

        let engine = NativeSecurityEngine::new(Some("test_machine_123"));
        let decrypted = engine.decrypt_text(python_ciphertext);
        assert_eq!(decrypted, expected_plaintext);
    }

    #[test]
    fn test_roundtrip_encryption_decryption() {
        let engine = NativeSecurityEngine::new(Some("custom_test_node_456"));
        let secret = "Confidential architectural documentation: AES-GCM-256 in Rust!";
        let enc = engine.encrypt_text(secret);
        assert!(enc.starts_with("enc::"));

        let dec = engine.decrypt_text(&enc);
        assert_eq!(dec, secret);
    }

    #[test]
    fn test_plain_text_passthrough() {
        let engine = NativeSecurityEngine::new(Some("test_node"));
        let normal = "Normal markdown content without encryption";
        assert_eq!(engine.decrypt_text(normal), normal);
    }

    #[test]
    fn test_tampered_or_invalid_key_handling() {
        let engine_a = NativeSecurityEngine::new(Some("machine_alpha"));
        let engine_b = NativeSecurityEngine::new(Some("machine_beta"));

        let secret = "Sensitive source code";
        let enc_a = engine_a.encrypt_text(secret);

        let dec_b = engine_b.decrypt_text(&enc_a);
        assert_eq!(dec_b, "[Protected Context Data - Hardware Key Mismatch]");
    }
}
