//! Token 加密工具（T-003）。
//!
//! - 算法：AES-256-GCM（对称加密 + 完整性校验）
//! - nonce：每次加密随机生成（12 字节）
//! - 存储格式：`base64(nonce || ciphertext)`，直接存入 `github_users.encrypted_token`
//! - 密钥：环境变量 `TOKEN_ENCRYPTION_KEY`（32 字节 hex，共 64 个 hex 字符）

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::errors::InfraError;

/// AES-256-GCM nonce 长度（96 bit）。
const NONCE_LEN: usize = 12;

/// Token 加解密器。
///
/// 由 `Config::from_env` 读取的 `TOKEN_ENCRYPTION_KEY` 构造，
/// 放入 `AppState` 供 service 层复用（T-004/T-007）。
#[derive(Clone)]
pub struct TokenCipher {
    cipher: Aes256Gcm,
}

// 不派生 Debug，避免意外打印密钥相关内部状态。
impl std::fmt::Debug for TokenCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenCipher(<redacted>)")
    }
}

#[allow(dead_code)] // T-004/T-007 启用
impl TokenCipher {
    /// 从 32 字节 hex 字符串（64 个 hex 字符）构造。
    pub fn from_hex_key(hex_key: &str) -> Result<Self, InfraError> {
        let key_bytes = hex::decode(hex_key.trim())
            .map_err(|_| InfraError::Crypto("TOKEN_ENCRYPTION_KEY is not valid hex".into()))?;
        if key_bytes.len() != 32 {
            return Err(InfraError::Crypto(format!(
                "TOKEN_ENCRYPTION_KEY must be 32 bytes (64 hex chars), got {} bytes",
                key_bytes.len()
            )));
        }
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|_| InfraError::Crypto("invalid AES-256 key length".into()))?;
        Ok(Self { cipher })
    }

    /// 加密明文 token，返回 `base64(nonce || ciphertext)`。
    pub fn encrypt(&self, plaintext: &str) -> Result<String, InfraError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| InfraError::Crypto("token encryption failed".into()))?;

        let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(combined))
    }

    /// 解密 `base64(nonce || ciphertext)`，返回明文 token。
    pub fn decrypt(&self, encoded: &str) -> Result<String, InfraError> {
        let combined = BASE64
            .decode(encoded.trim())
            .map_err(|_| InfraError::Crypto("encrypted token is not valid base64".into()))?;
        if combined.len() <= NONCE_LEN {
            return Err(InfraError::Crypto(
                "encrypted token payload too short".into(),
            ));
        }
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
        let nonce_arr: [u8; NONCE_LEN] = nonce_bytes
            .try_into()
            .map_err(|_| InfraError::Crypto("invalid nonce length".into()))?;
        let nonce = Nonce::from(nonce_arr);

        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| InfraError::Crypto("token decryption failed".into()))?;
        String::from_utf8(plaintext)
            .map_err(|_| InfraError::Crypto("decrypted token is not valid utf-8".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn roundtrip() {
        let cipher = TokenCipher::from_hex_key(TEST_KEY).unwrap();
        let token = "ghp_example_token_1234567890";
        let encrypted = cipher.encrypt(token).unwrap();
        assert_ne!(encrypted, token);
        assert_eq!(cipher.decrypt(&encrypted).unwrap(), token);
    }

    #[test]
    fn nonce_is_random_per_encryption() {
        let cipher = TokenCipher::from_hex_key(TEST_KEY).unwrap();
        let a = cipher.encrypt("same-token").unwrap();
        let b = cipher.encrypt("same-token").unwrap();
        assert_ne!(a, b, "random nonce must produce different ciphertexts");
        assert_eq!(cipher.decrypt(&a).unwrap(), "same-token");
        assert_eq!(cipher.decrypt(&b).unwrap(), "same-token");
    }

    #[test]
    fn rejects_bad_key() {
        assert!(TokenCipher::from_hex_key("not-hex").is_err());
        assert!(TokenCipher::from_hex_key("0102").is_err());
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        let cipher = TokenCipher::from_hex_key(TEST_KEY).unwrap();
        let encrypted = cipher.encrypt("secret").unwrap();
        let mut raw = BASE64.decode(&encrypted).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        let tampered = BASE64.encode(raw);
        assert!(
            cipher.decrypt(&tampered).is_err(),
            "GCM must detect tampering"
        );
    }

    #[test]
    fn rejects_garbage_input() {
        let cipher = TokenCipher::from_hex_key(TEST_KEY).unwrap();
        assert!(cipher.decrypt("!!not-base64!!").is_err());
        assert!(cipher.decrypt("c2hvcnQ=").is_err()); // 解码后不足 nonce 长度
    }
}
