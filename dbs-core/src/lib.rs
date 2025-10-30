//! core functionality for encrypting and decrypting
//! save files from "Death by Scrolling"
//!
//! # Modules
//!
//! - `crypto`: XXTEA/BTEA encryption and decryption
//! - `codec`: Save file format encoding/decoding

pub mod crypto;
pub mod codec;

// Re-export commonly used items
pub use codec::{
    decrypt, encrypt, calc_checksum, pack_block, unpack_block,
    UnpackedBlock, KEY, EXTRA4,
};
pub use crypto::{
    xxtea_encrypt_bytes, xxtea_decrypt_bytes,
    xxtea_encrypt_block, xxtea_decrypt_block,
};

