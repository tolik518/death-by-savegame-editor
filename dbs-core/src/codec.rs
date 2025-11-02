/// Handles the payload packing/unpacking with checksum, extra4, and padding
use anyhow::{Context, Result, bail};

/// 16-Byte XXTEA/BTEA Key (little endian, from game dump)
pub const KEY: [u8; 16] = [
    0x93, 0x9d, 0xab, 0x7a, 0x2a, 0x56, 0xf8, 0xaf, 0xb4, 0xdb, 0xa9, 0xb5, 0x22, 0xa3, 0x4b, 0x2b,
];

/// extra4 field in the footer
pub const EXTRA4: u32 = 0x0169027d;

/// Layout: [payload | checksum(4 LE) | extra4(4 LE) | pad(0..7) | padlen(1)]
pub struct UnpackedBlock {
    pub payload: Vec<u8>,
    pub checksum: u32,
    pub extra4: u32,
    pub padlen: u8,
}

/// Calculate checksum for a payload
/// Engine formula: 0x06583463 + sum of all payload bytes (mod 2^32)
pub fn calc_checksum(payload: &[u8]) -> u32 {
    let sum: u64 = payload.iter().map(|&b| b as u64).sum();
    ((0x06583463u64 + sum) & 0xFFFFFFFF) as u32
}

/// Pack payload into a plaintext block ready for encryption
/// The block length must be a multiple of 8 (engine requirement)
pub fn pack_block(payload: &[u8], extra4: u32, pad_bytes: Option<&[u8]>) -> Result<Vec<u8>> {
    // base = payload + checksum(4) + extra4(4) + padlen(1)
    let base = payload.len() + 9;

    // Find smallest padlen (0..7) that makes total_len % 8 == 0
    let padlen = (8 - (base % 8)) % 8;

    let checksum = calc_checksum(payload);

    // Build the block
    let mut block = Vec::with_capacity(payload.len() + 9 + padlen);
    block.extend_from_slice(payload);
    block.extend_from_slice(&checksum.to_le_bytes());
    block.extend_from_slice(&extra4.to_le_bytes());

    // Add padding
    if padlen > 0 {
        if let Some(pad) = pad_bytes {
            if pad.len() < padlen {
                bail!("pad_bytes too short");
            }
            block.extend_from_slice(&pad[..padlen]);
        } else {
            // Use deterministic zero bytes (no RNG needed)
            block.resize(block.len() + padlen, 0);
        }
    }

    // Add padlen byte
    block.push(padlen as u8);

    // Sanity check: engine requires multiple of 8
    if block.len() % 8 != 0 {
        bail!("plain block must be multiple of 8, got {}", block.len());
    }

    Ok(block)
}

/// Unpack a plaintext block into payload and metadata
pub fn unpack_block(plain: &[u8]) -> Result<UnpackedBlock> {
    if plain.len() < 9 {
        bail!("Block too short: {} bytes", plain.len());
    }

    let padlen = plain[plain.len() - 1];

    if padlen > 8 {
        bail!("padlen out of range: {}", padlen);
    }

    let payload_len = plain.len() as i64 - padlen as i64 - 9;
    if payload_len < 0 {
        bail!("payload_len < 0 (corrupt?)");
    }

    let payload_len = payload_len as usize;
    let payload = plain[..payload_len].to_vec();

    // Extract checksum and extra4 (both little endian u32)
    let checksum = u32::from_le_bytes([
        plain[payload_len],
        plain[payload_len + 1],
        plain[payload_len + 2],
        plain[payload_len + 3],
    ]);

    let extra4 = u32::from_le_bytes([
        plain[payload_len + 4],
        plain[payload_len + 5],
        plain[payload_len + 6],
        plain[payload_len + 7],
    ]);

    Ok(UnpackedBlock {
        payload,
        checksum,
        extra4,
        padlen,
    })
}

/// Decrypt a ciphertext save file to plaintext payload
pub fn decrypt(cipher: &[u8]) -> Result<UnpackedBlock> {
    use crate::crypto::xxtea_decrypt_bytes;

    if cipher.len() % 8 != 0 {
        eprintln!(
            "[warn] cipher len {} is not a multiple of 8 – engine would reject this.",
            cipher.len()
        );
    }

    let plain = xxtea_decrypt_bytes(cipher, &KEY);
    let unpacked = unpack_block(&plain).context("Failed to unpack block")?;

    let calc = calc_checksum(&unpacked.payload);
    if unpacked.checksum != calc {
        eprintln!(
            "[warn] checksum mismatch: stored=0x{:08x} calc=0x{:08x}",
            unpacked.checksum, calc
        );
    }

    Ok(unpacked)
}

/// Encrypt a plaintext payload to ciphertext save file
pub fn encrypt(payload: &[u8]) -> Result<Vec<u8>> {
    use crate::crypto::xxtea_encrypt_bytes;

    // Build plaintext block with proper padding
    let block = pack_block(payload, EXTRA4, None)?;

    // Encrypt
    let enc = xxtea_encrypt_bytes(&block, &KEY);

    // Sanity check
    if enc.len() % 8 != 0 {
        bail!("cipher must be multiple of 8, got {}", enc.len());
    }

    Ok(enc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_checksum() {
        let payload = b"test";
        let sum = calc_checksum(payload);
        // 0x06583463 + (116 + 101 + 115 + 116) = 0x06583463 + 448 = 0x06583623
        assert_eq!(sum, 0x06583623);
    }

    #[test]
    fn test_pack_unpack() {
        let payload = b"Hello, World!";
        let block = pack_block(payload, EXTRA4, None).unwrap();
        assert_eq!(block.len() % 8, 0);

        let unpacked = unpack_block(&block).unwrap();
        assert_eq!(unpacked.payload, payload);
        assert_eq!(unpacked.extra4, EXTRA4);
        assert_eq!(unpacked.checksum, calc_checksum(payload));
    }

    /// Helper function to test a single save file with full round-trip
    fn test_save_file_roundtrip(save_path: &Path) -> anyhow::Result<()> {
        let save_name = save_path.file_name().unwrap().to_string_lossy();

        // Read the original encrypted save
        let original_cipher = std::fs::read(save_path)?;

        // Decrypt
        let unpacked = decrypt(&original_cipher)?;

        // Verify checksum
        let calc = calc_checksum(&unpacked.payload);
        assert_eq!(
            unpacked.checksum, calc,
            "Checksum mismatch for {}: stored=0x{:08x} calc=0x{:08x}",
            save_name, unpacked.checksum, calc
        );

        // Re-encrypt
        let new_cipher = encrypt(&unpacked.payload)?;

        // Decrypt again
        let unpacked2 = decrypt(&new_cipher)?;

        // Verify the payload is identical after round-trip
        assert_eq!(
            unpacked.payload, unpacked2.payload,
            "Payload mismatch after round-trip for {}",
            save_name
        );

        // Verify checksums match
        assert_eq!(
            unpacked.checksum, unpacked2.checksum,
            "Checksum changed after round-trip for {}",
            save_name
        );

        Ok(())
    }

    #[test]
    fn test_all_saves_roundtrip() {
        // Path to saves directory relative to workspace root
        let saves_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data")
            .join("saves");

        if !saves_dir.exists() {
            println!("Warning: saves directory not found at {:?}", saves_dir);
            return;
        }

        let mut tested = 0;
        let mut failed = Vec::new();

        // Read all files in saves directory
        let entries = std::fs::read_dir(&saves_dir).expect("Failed to read saves directory");

        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            // Only test files (skip directories)
            if !path.is_file() {
                continue;
            }

            let name = path.file_name().unwrap().to_string_lossy().to_string();

            println!("Testing save file: {}", name);

            match test_save_file_roundtrip(&path) {
                Ok(_) => {
                    println!("  ✓ {}: PASSED", name);
                    tested += 1;
                }
                Err(e) => {
                    println!("  ✗ {}: FAILED - {:?}", name, e);
                    failed.push((name, e));
                }
            }
        }

        println!("\n=== Test Summary ===");
        println!("Tested: {}", tested);
        println!("Passed: {}", tested - failed.len());
        println!("Failed: {}", failed.len());

        if !failed.is_empty() {
            println!("\nFailed saves:");
            for (name, err) in &failed {
                println!("  - {}: {:?}", name, err);
            }
            panic!("{} save file(s) failed round-trip test", failed.len());
        }

        assert!(tested > 0, "No save files were tested!");
    }

    #[test]
    fn test_0gems() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/saves/0gems");
        if path.exists() {
            test_save_file_roundtrip(&path).expect("0gems test failed");
        }
    }

    #[test]
    fn test_42gems() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/saves/42gems");
        if path.exists() {
            test_save_file_roundtrip(&path).expect("42gems test failed");
        }
    }

    #[test]
    fn test_13337gems() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/saves/13337gems");
        if path.exists() {
            test_save_file_roundtrip(&path).expect("13337gems test failed");
        }
    }

    #[test]
    fn test_424243gems() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/saves/424243gems");
        if path.exists() {
            test_save_file_roundtrip(&path).expect("424243gems test failed");
        }
    }
}
