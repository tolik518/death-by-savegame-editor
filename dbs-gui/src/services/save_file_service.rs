use anyhow::Result;
use dbs_core::{decrypt, encrypt};
use std::path::Path;

/// Service for handling save file encryption/decryption operations
pub struct SaveFileService;

impl SaveFileService {
    pub fn new() -> Self {
        Self
    }

    /// Decrypt a save file and return the content as a string
    pub fn load(&self, path: &Path) -> Result<String> {
        let cipher = std::fs::read(path)?;
        let unpacked = decrypt(&cipher)?;
        let content = String::from_utf8_lossy(&unpacked.payload).to_string();
        Ok(content)
    }

    /// Encrypt and save content to a file
    pub fn save(&self, content: &str, path: &Path) -> Result<()> {
        let cipher = encrypt(content.as_bytes())?;
        std::fs::write(path, cipher)?;
        Ok(())
    }
}

impl Default for SaveFileService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_roundtrip() {
        let service = SaveFileService::new();
        let temp_file = NamedTempFile::new().unwrap();
        let test_content = "test: 123";

        // Save
        service.save(test_content, temp_file.path()).unwrap();

        // Load
        let loaded = service.load(temp_file.path()).unwrap();
        assert_eq!(loaded, test_content);
    }
}

