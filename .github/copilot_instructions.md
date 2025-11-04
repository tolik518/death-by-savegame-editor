# Death by Savegame Editor - Developer Instructions

## System Overview

Rust workspace with three crates:
- **dbs-core**: XXTEA encryption/decryption library
- **dbs-cli**: Command-line interface
- **dbs-gui**: GUI with egui (form-based HOCON editor + raw text fallback)

## Core Architecture

### Save File Format
```
[payload | checksum(4 LE) | extra4(4 LE) | pad(0..7) | padlen(1)]
```

- **Checksum**: `(0x06583463 + sum_of_payload_bytes) mod 2^32`
- **Extra4**: `0x0169027d` (magic constant)
- **Block size**: Must be multiple of 8 bytes (auto-padded)

### Key Constants (dbs-core/src/codec.rs)
```rust
pub const KEY: [u8; 16] = [0x93, 0x9d, 0xab, 0x7a, 0x2a, 0x56, 0xf8, 0xaf, 
                           0xb4, 0xdb, 0xa9, 0xb5, 0x22, 0xa3, 0x4b, 0x2b];
pub const EXTRA4: u32 = 0x0169027d;
```

### XXTEA Encryption
- All values are **little-endian**
- Block size: variable (minimum 2 u32 values)
- Rounds: `6 + 52/n` where n = block length

### Core API
```rust
// High-level functions (dbs-core/src/codec.rs)
pub fn encrypt(payload: &[u8]) -> Result<Vec<u8>>
pub fn decrypt(cipher: &[u8]) -> Result<UnpackedBlock>
```

## GUI Features

### State Machine (editor_state.rs)
- **Welcome**: Main menu
- **Editing**: Raw text editor (preserves original content with quotes)
- **EditingParsed**: Form-based HOCON editor (primary mode)
- **SelectingBackup**: Backup list
- **ConfirmRestore**: Restore confirmation
- **Error**: Error display

### HOCON Parser (hocon_parser.rs)
- Custom parser for game's HOCON format
- Types: `Int`, `Float`, `String`, `Bool`, `Object`, `Array`
- Uses `IndexMap` to preserve field order
- Auto-fallback to raw mode if parsing fails

### Form Editor (hocon_editor.rs)
- 11 predefined categories (Player Stats, Achievements, Challenges, etc.)
- Type-aware widgets with icons (🔢 📝 ☑️ 📁 📋)
- Search in keys AND values with highlighting (yellow=key, green=value)
- Smart array display:
  - String arrays: Compact inline
  - Number arrays: Grid layout
  - Object arrays: Card view
  - Mixed arrays: Type icons
- Toggle between Form ↔ Raw modes (preserves edits)

### Backup System (backup.rs)
- Backup created when user clicks Save (copies original encrypted save.bin BEFORE overwriting)
- One backup per session: `backup/YYYY-MM-DD_HH-MM-SS.bak`
- Backups stored in dedicated `backup/` subfolder within save directory
- Session resets when returning to Welcome screen
- Emergency backup before restore: `backup/emergency_before_restore_YYYY-MM-DD_HH-MM-SS.bak` (timestamped to prevent overwriting)
- Backup validation: Attempts to decrypt each backup to verify integrity
- Backup list with restore functionality (invalid backups shown in red, cannot be restored)

### UI Features
- **Keyboard shortcuts**:
  - `Ctrl+S`: Save changes
  - `Escape`: Clear search
- **Status bar**: Always-visible footer at bottom
- **Theme toggle**: Dark/light mode (🌙 ☀)
- **Dynamic window sizing**: Auto-resizes based on state
- **Window size debug**: Prints dimensions to terminal on resize

## Building

```bash
# Development
cargo build

# Release (Linux)
cargo build --release

# Release (Windows cross-compile from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# Run GUI
./target/release/dbs-gui

# Run CLI
./target/release/dbs-cli decrypt save.bin save.hocon
./target/release/dbs-cli encrypt save.hocon save.bin

# Tests
cargo test
```

## Key Design Decisions

1. **Custom HOCON parser**: Smaller binary, better control
2. **Dual editor modes**: Form (primary) + Raw (fallback) with bidirectional switching
3. **Raw mode preserves original**: Shows actual file content with quotes and formatting
4. **Edit preservation**: Switching Form ↔ Raw preserves all changes
5. **Smart backup strategy**: One backup per session, created only when user saves (prevents backup spam)
6. **Clean architecture**: Domain → Application → Presentation → Infrastructure

## Common Extension Points

### Add New HOCON Category
```rust
// In hocon_editor.rs, add to get_categories()
Category::new(
    "Category Name",
    "🎯",  // Icon
    vec!["field1", "field2"],
)
```

### Add New Value Type
```rust
// In hocon_parser.rs
pub enum HoconValue {
    // ...existing types...
    NewType(SomeType), // Your new type
}
```

### Change Window Size for State
```rust
// In main.rs, ideal_window_size_for_state()
EditorState::YourState => egui::vec2(width, height),
```

## Testing

```bash
cargo test -p dbs-core     # 3 tests (encryption/decryption)
cargo test -p dbs-gui      # 7 tests (parser + backups)
cargo test                 # All tests
```

## Dependencies

```toml
anyhow = "1.0"      # Error handling
egui = "0.33"       # GUI framework
indexmap = "2.0"    # Ordered maps (HOCON)
chrono = "0.4"      # Timestamps (backups)
clap = "4.5"        # CLI parsing
rfd = "0.15"        # File dialogs
```

## Save File Locations

- **Linux**: `~/.local/share/Terrible Toybox/Death by Scrolling/`
- **Windows**: `%LOCALAPPDATA%\Terrible Toybox\Death by Scrolling\`
- **macOS**: `~/Library/Application Support/Terrible Toybox/Death by Scrolling/`

## Important Implementation Details

### Raw ↔ Form Mode Switching
- **Raw → Form**: Parses current edited content (not original)
- **Form → Raw**: Serializes document to HOCON
- **Original content preserved**: `original_content` field maintains raw file
- **Edits preserved**: All changes carry over when switching modes

### Search Feature
- Searches both keys and values recursively
- Yellow highlight: Key match
- Green highlight: Value match
- Shows match count with breakdown

### Array Rendering
Automatically detects array type:
- Homogeneous strings → Inline compact view
- Homogeneous numbers → Grid with drag controls
- All objects → Card-based layout
- Mixed types → Type icons per item

### Footer (Status Bar)
- Implemented as `TopBottomPanel::bottom()`
- Always visible at bottom
- Shows author + status messages

