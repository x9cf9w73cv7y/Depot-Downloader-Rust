use anyhow::{Result, Context};
use aes::cipher::{KeyIvInit, StreamCipher};
use ctr::cipher::generic_array::GenericArray;
use std::collections::HashMap;

pub type Aes128Ctr = ctr::Ctr128LE<aes::Aes128>;

pub struct ManifestDecryptor {
    depot_keys: HashMap<u32, Vec<u8>>,
}

impl ManifestDecryptor {
    pub fn new() -> Self {
        Self {
            depot_keys: HashMap::new(),
        }
    }

    pub fn add_depot_key(&mut self, depot_id: u32, key: Vec<u8>) {
        tracing::info!("Adding depot key for depot {}", depot_id);
        self.depot_keys.insert(depot_id, key);
    }

    pub fn add_depot_keys_from_string(&mut self, keys_str: &str) -> Result<()> {
        // Parse format: "depotId;hexKey\ndepotId;hexKey\n..."
        for line in keys_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ';').collect();
            if parts.len() != 2 {
                tracing::warn!("Invalid depot key format: {}", line);
                continue;
            }

            let depot_id: u32 = parts[0].parse()
                .context(format!("Invalid depot ID: {}", parts[0]))?;
            
            let key_hex = parts[1].trim();
            let key = Self::hex_to_bytes(key_hex)?;
            
            if key.len() != 16 {
                tracing::warn!(
                    "Invalid key length for depot {}: expected 16 bytes, got {}",
                    depot_id, key.len()
                );
                continue;
            }

            self.add_depot_key(depot_id, key);
        }

        Ok(())
    }

    pub fn get_depot_key(&self, depot_id: u32) -> Option<&Vec<u8>> {
        self.depot_keys.get(&depot_id)
    }

    pub fn decrypt_manifest(&self, depot_id: u32, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let key = self.depot_keys.get(&depot_id)
            .ok_or_else(|| anyhow::anyhow!("No decryption key for depot {}", depot_id))?;

        // Steam uses AES-128-CTR
        // The IV is typically derived from the manifest ID or is zero
        // For Steam, it's usually a 16-byte IV (often zeros or derived from manifest)
        let iv = [0u8; 16]; // Steam typically uses zero IV

        let mut cipher = Aes128Ctr::new(
            GenericArray::from_slice(key),
            GenericArray::from_slice(&iv)
        );

        let mut decrypted = encrypted_data.to_vec();
        cipher.apply_keystream(&mut decrypted);

        tracing::debug!("Decrypted {} bytes for depot {}", decrypted.len(), depot_id);
        Ok(decrypted)
    }

    pub fn decrypt_chunk(&self, depot_id: u32, chunk_data: &[u8], iv: &[u8; 16]) -> Result<Vec<u8>> {
        let key = self.depot_keys.get(&depot_id)
            .ok_or_else(|| anyhow::anyhow!("No decryption key for depot {}", depot_id))?;

        let mut cipher = Aes128Ctr::new(
            GenericArray::from_slice(key),
            GenericArray::from_slice(iv)
        );

        let mut decrypted = chunk_data.to_vec();
        cipher.apply_keystream(&mut decrypted);

        Ok(decrypted)
    }

    fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
        let hex = hex.replace(" ", "").replace("-", "");
        
        if hex.len() % 2 != 0 {
            return Err(anyhow::anyhow!("Invalid hex string length"));
        }

        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for i in (0..hex.len()).step_by(2) {
            let byte_str = &hex[i..i+2];
            let byte = u8::from_str_radix(byte_str, 16)
                .context(format!("Invalid hex byte: {}", byte_str))?;
            bytes.push(byte);
        }

        Ok(bytes)
    }

    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for ManifestDecryptor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_conversion() {
        let hex = "0123456789abcdef";
        let bytes = ManifestDecryptor::hex_to_bytes(hex).unwrap();
        assert_eq!(bytes, vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
        
        let back = ManifestDecryptor::bytes_to_hex(&bytes);
        assert_eq!(back, hex);
    }

    #[test]
    fn test_parse_depot_keys() {
        let mut decryptor = ManifestDecryptor::new();
        let keys = "12345;0123456789abcdef0123456789abcdef\n67890;fedcba9876543210fedcba9876543210";
        
        decryptor.add_depot_keys_from_string(keys).unwrap();
        
        assert!(decryptor.get_depot_key(12345).is_some());
        assert!(decryptor.get_depot_key(67890).is_some());
        assert!(decryptor.get_depot_key(99999).is_none());
    }
}
