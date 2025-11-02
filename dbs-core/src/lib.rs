//! core functionality for encrypting and decrypting
//! save files from "Death by Scrolling"
//!
//! # Modules
//!
//! - `crypto`: XXTEA/BTEA encryption and decryption
//! - `codec`: Save file format encoding/decoding

pub mod codec;
pub mod crypto;

// Re-export commonly used items
pub use codec::{
    EXTRA4, KEY, UnpackedBlock, calc_checksum, decrypt, encrypt, pack_block, unpack_block,
};
pub use crypto::{
    xxtea_decrypt_block, xxtea_decrypt_bytes, xxtea_encrypt_block, xxtea_encrypt_bytes,
};
