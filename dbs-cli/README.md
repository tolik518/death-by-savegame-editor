# dbs-cli

A CLI tool for encrypting and decrypting [Death by Scrolling](https://store.steampowered.com/app/3773590/Death_by_Scrolling/) save files.

## Installation

Build from source:
```bash
cargo build --release --bin dbs-cli
```

The binary will be located at `target/release/dbs-cli`.

## Usage

### Decrypt a save file

Example:
```bash
dbs-cli decrypt ~/.local/share/"Terrible Toybox"/"Death by Scrolling"/save.bin payload.hocon
```

### Encrypt a payload

```bash
dbs-cli encrypt <INPUT_HOCON> <OUTPUT_SAVE>
```

Example:
```bash
dbs-cli encrypt payload.hocon save.bin
```

### Help

```bash
dbs-cli --help
dbs-cli decrypt --help
dbs-cli encrypt --help
```

## Example

```bash
export SAVE_DIR="$HOME/.local/share/Terrible Toybox/Death by Scrolling"

# Decrypt
dbs-cli decrypt "$SAVE_DIR/save.bin" payload.hocon

# Edit
nano payload.hocon

# Encrypt
dbs-cli encrypt payload.hocon "$SAVE_DIR/save.bin"

# Disable leaderboards
sed -i 's/globalLeaderboards: 1/globalLeaderboards: 0/' "$SAVE_DIR/Prefs.json"
```
