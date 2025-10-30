# dbs-core

Core lib for Death by Savegame Editor.

## API Documentation

### Example: Decrypt a save file

```rust
use dbs_core::decrypt;
use std::fs;

fn main() -> anyhow::Result<()> {
    let cipher = fs::read("save.bin")?;
    let unpacked = decrypt(&cipher)?;
    
    println!("Payload size: {} bytes", unpacked.payload.len());
    println!("Checksum: 0x{:08x}", unpacked.checksum);
    println!("Extra4: 0x{:08x}", unpacked.extra4);
    
    fs::write("payload.hocon", &unpacked.payload)?;
    Ok(())
}
```

### Example: Encrypt a payload

```rust
use dbs_core::encrypt;
use std::fs;

fn main() -> anyhow::Result<()> {
    let payload = fs::read("payload.hocon")?;
    let cipher = encrypt(&payload)?;
    
    fs::write("save.bin", &cipher)?;
    Ok(())
}
```

### Main Functions

#### `decrypt(cipher: &[u8]) -> Result<UnpackedBlock>`
Decrypts a save file and unpacks it into payload and metadata.

#### `encrypt(payload: &[u8]) -> Result<Vec<u8>>`
Encrypts a payload into a save file format.

#### `calc_checksum(payload: &[u8]) -> u32`
Calculates the checksum for a payload using the game's algorithm.

### Types

#### `UnpackedBlock`
```rust
pub struct UnpackedBlock {
    pub payload: Vec<u8>,
    pub checksum: u32,
    pub extra4: u32,
    pub padlen: u8,
}
```

### Constants

#### `KEY: [u8; 16]`
```hex
93 9d ab 7a 2a 56 f8 af b4 db a9 b5 22 a3 4b 2b
```
The hardcoded 128-bit encryption key extracted from the game.

#### `EXTRA4: u32`
Footer field constant used by the game (0x0169027d).

## Technical Details

### XXTEA Algorithm
- Block cipher with variable block size
- 128-bit key
- Delta constant: 0x9E3779B9
- Rounds: 6 + 52/n (where n is block size)

### Save File Format
```
[payload | checksum(4 LE) | extra4(4 LE) | pad(0..7) | padlen(1)]
```

- All fields use little-endian byte order
- Padding ensures total length is a multiple of 8
- Checksum: `0x06583463 + sum(payload bytes)` mod 2^32

## Testing

```bash
cargo test
```

