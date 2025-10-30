/// XXTEA/BTEA encryption and decryption implementation

const DELTA: u32 = 0x9E3779B9;

/// Wraps a value to u32
#[inline]
fn u32_wrap(x: u64) -> u32 {
    (x & 0xFFFFFFFF) as u32
}

/// XXTEA encryption for a block of u32 values
pub fn xxtea_encrypt_block(v: &mut [u32], k: &[u32; 4]) {
    let n = v.len();
    if n < 2 {
        return;
    }

    let rounds = 6 + 52 / n;
    let mut z = v[n - 1];
    let mut y;
    let mut sum: u32 = 0;

    for _ in 0..rounds {
        sum = sum.wrapping_add(DELTA);
        let e = ((sum >> 2) & 3) as usize;

        for p in 0..n - 1 {
            y = v[p + 1];
            let mx = u32_wrap(
                (u32_wrap(((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64) as u64)
                    ^ ((sum ^ y) as u64 + (k[(p & 3) ^ e] ^ z) as u64)
            );
            v[p] = v[p].wrapping_add(mx);
            z = v[p];
        }

        y = v[0];
        let mx = u32_wrap(
            (u32_wrap(((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64) as u64)
                ^ ((sum ^ y) as u64 + (k[((n - 1) & 3) ^ e] ^ z) as u64)
        );
        v[n - 1] = v[n - 1].wrapping_add(mx);
        z = v[n - 1];
    }
}

/// XXTEA decryption for a block of u32 values
pub fn xxtea_decrypt_block(v: &mut [u32], k: &[u32; 4]) {
    let n = v.len();
    if n < 2 {
        return;
    }

    let rounds = 6 + 52 / n;
    let mut sum = u32_wrap((rounds as u32).wrapping_mul(DELTA) as u64);
    let mut z;
    let mut y = v[0];

    for _ in 0..rounds {
        let e = ((sum >> 2) & 3) as usize;

        for p in (1..n).rev() {
            z = v[p - 1];
            let mx = u32_wrap(
                (u32_wrap(((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64) as u64)
                    ^ ((sum ^ y) as u64 + (k[(p & 3) ^ e] ^ z) as u64)
            );
            v[p] = v[p].wrapping_sub(mx);
            y = v[p];
        }

        z = v[n - 1];
        let mx = u32_wrap(
            (u32_wrap(((z >> 5) ^ (y << 2)) as u64 + ((y >> 3) ^ (z << 4)) as u64) as u64)
                ^ ((sum ^ y) as u64 + (k[(0 & 3) ^ e] ^ z) as u64)
        );
        v[0] = v[0].wrapping_sub(mx);
        y = v[0];

        sum = sum.wrapping_sub(DELTA);
    }
}

/// Encrypts bytes using XXTEA with a 16-byte key (little endian)
pub fn xxtea_encrypt_bytes(data: &[u8], key16_le: &[u8; 16]) -> Vec<u8> {
    // Pad to multiple of 4 bytes
    let pad = (4 - (data.len() % 4)) % 4;
    let mut bb = data.to_vec();
    bb.resize(data.len() + pad, 0);

    // Convert bytes to u32 array (little endian)
    let mut v: Vec<u32> = bb
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    // If only one u32, append a zero (XXTEA requirement)
    if v.len() == 1 {
        v.push(0);
    }

    // Convert key to u32 array (little endian)
    let k: [u32; 4] = [
        u32::from_le_bytes([key16_le[0], key16_le[1], key16_le[2], key16_le[3]]),
        u32::from_le_bytes([key16_le[4], key16_le[5], key16_le[6], key16_le[7]]),
        u32::from_le_bytes([key16_le[8], key16_le[9], key16_le[10], key16_le[11]]),
        u32::from_le_bytes([key16_le[12], key16_le[13], key16_le[14], key16_le[15]]),
    ];

    // Encrypt
    xxtea_encrypt_block(&mut v, &k);

    // Convert back to bytes
    let mut result = Vec::new();
    for val in v {
        result.extend_from_slice(&val.to_le_bytes());
    }

    // Return only the original padded length
    result.truncate(bb.len());
    result
}

/// Decrypts bytes using XXTEA with a 16-byte key (little endian)
pub fn xxtea_decrypt_bytes(data: &[u8], key16_le: &[u8; 16]) -> Vec<u8> {
    // Pad to multiple of 4 bytes
    let pad = (4 - (data.len() % 4)) % 4;
    let mut bb = data.to_vec();
    bb.resize(data.len() + pad, 0);

    // Convert bytes to u32 array (little endian)
    let mut v: Vec<u32> = bb
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    // If only one u32, append a zero (XXTEA requirement)
    if v.len() == 1 {
        v.push(0);
    }

    // Convert key to u32 array (little endian)
    let k: [u32; 4] = [
        u32::from_le_bytes([key16_le[0], key16_le[1], key16_le[2], key16_le[3]]),
        u32::from_le_bytes([key16_le[4], key16_le[5], key16_le[6], key16_le[7]]),
        u32::from_le_bytes([key16_le[8], key16_le[9], key16_le[10], key16_le[11]]),
        u32::from_le_bytes([key16_le[12], key16_le[13], key16_le[14], key16_le[15]]),
    ];

    // Decrypt
    xxtea_decrypt_block(&mut v, &k);

    // Convert back to bytes
    let mut result = Vec::new();
    for val in v {
        result.extend_from_slice(&val.to_le_bytes());
    }

    // Return only the original data length (before padding)
    result.truncate(data.len());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0u8; 16];
        let data = b"Hello, World!";

        let encrypted = xxtea_encrypt_bytes(data, &key);
        let decrypted = xxtea_decrypt_bytes(&encrypted, &key);

        // XXTEA preserves the padded length - the decrypted data will include
        // the padding bytes that were added during encryption (to make it a multiple of 4)
        assert_eq!(&decrypted[..data.len()], data);
    }
}

