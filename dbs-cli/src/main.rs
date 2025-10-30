use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use dbs_core::{decrypt, encrypt, calc_checksum};

#[derive(Parser)]
#[command(name = "dbs-cli")]
#[command(about = "Death by Scrolling save (de|en)crypt – CLI tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decrypt cipher to plaintext payload
    Decrypt {
        /// Path to the encrypted save file
        cipher: PathBuf,

        /// Path to write the decrypted plaintext payload
        out_plain: PathBuf,
    },

    /// Encrypt plaintext payload to cipher
    Encrypt {
        /// Path to the plaintext payload file
        plain: PathBuf,

        /// Path to write the encrypted cipher file
        out_cipher: PathBuf,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decrypt { cipher, out_plain } => {
            cmd_decrypt(&cipher, &out_plain)?;
        }
        Commands::Encrypt { plain, out_cipher } => {
            cmd_encrypt(&plain, &out_cipher)?;
        }
    }

    Ok(())
}

fn cmd_decrypt(cipher_path: &PathBuf, out_plain_path: &PathBuf) -> Result<()> {
    // Read encrypted file
    let enc = fs::read(cipher_path)
        .with_context(|| format!("Failed to read cipher file: {}", cipher_path.display()))?;

    println!("[info] len(enc)={}", enc.len());

    // Decrypt
    let unpacked = decrypt(&enc)?;

    println!("[info] padlen={}  extra4=0x{:08x}", unpacked.padlen, unpacked.extra4);

    let calc = calc_checksum(&unpacked.payload);
    println!(
        "[info] checksum stored=0x{:08x}  calc=0x{:08x}  -> {}",
        unpacked.checksum,
        calc,
        if unpacked.checksum == calc { "OK" } else { "MISMATCH" }
    );

    // Write decrypted payload
    fs::write(out_plain_path, &unpacked.payload)
        .with_context(|| format!("Failed to write plaintext file: {}", out_plain_path.display()))?;

    println!("[ok] wrote payload -> {}", out_plain_path.display());

    Ok(())
}

fn cmd_encrypt(plain_path: &PathBuf, out_cipher_path: &PathBuf) -> Result<()> {
    // Read plaintext payload
    let payload = fs::read(plain_path)
        .with_context(|| format!("Failed to read plaintext file: {}", plain_path.display()))?;

    // Encrypt
    let enc = encrypt(&payload)?;

    // Write encrypted file
    fs::write(out_cipher_path, &enc)
        .with_context(|| format!("Failed to write cipher file: {}", out_cipher_path.display()))?;

    println!("[ok] wrote encrypted block -> {}", out_cipher_path.display());

    Ok(())
}

