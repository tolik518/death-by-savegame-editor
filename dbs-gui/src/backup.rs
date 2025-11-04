use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local};
use dbs_core::decrypt;
use std::fs;
use std::path::{Path, PathBuf};

pub struct BackupManager {
    save_dir: PathBuf,
    backup_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub filename: String,
    pub timestamp: DateTime<Local>,
    pub size: u64,
    pub is_valid: bool,
}

impl BackupManager {
    pub fn new(save_dir: PathBuf) -> Self {
        let backup_dir = save_dir.join("backup");
        Self { save_dir, backup_dir }
    }

    /// Lists all .bak files sorted by timestamp (newest first)
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        for entry in fs::read_dir(&self.backup_dir).context("Failed to read backup directory")? {
            let entry = entry?;
            let path = entry.path();

            // Only process .bak files
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("bak") {
                continue;
            }

            let metadata = fs::metadata(&path)?;
            let modified = metadata.modified()?;
            let timestamp: DateTime<Local> = modified.into();

            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Validate backup by attempting to decrypt
            let is_valid = Self::validate_backup(&path);

            backups.push(BackupInfo {
                path,
                filename,
                timestamp,
                size: metadata.len(),
                is_valid,
            });
        }

        // Sort by timestamp, newest first
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(backups)
    }

    /// Validates a backup file by attempting to decrypt it
    fn validate_backup(path: &Path) -> bool {
        if let Ok(cipher) = fs::read(path) {
            decrypt(&cipher).is_ok()
        } else {
            false
        }
    }

    /// Creates a timestamped backup of the source file
    /// Format: YYYY-MM-DD_HH-MM-SS.bak
    pub fn create_backup(&self, source_path: &Path) -> Result<PathBuf> {
        if !source_path.exists() {
            bail!("Source file does not exist: {}", source_path.display());
        }

        // Ensure backup directory exists
        fs::create_dir_all(&self.backup_dir).context("Failed to create backup directory")?;

        // Generate timestamp filename
        let now = Local::now();
        let filename = format!("{}.bak", now.format("%Y-%m-%d_%H-%M-%S"));
        let backup_path = self.backup_dir.join(filename);

        // Copy file
        fs::copy(source_path, &backup_path).context("Failed to create backup")?;

        Ok(backup_path)
    }

    /// Restores a backup to save.bin
    pub fn restore_backup(&self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            bail!("Backup file does not exist: {}", backup_path.display());
        }

        let save_path = self.get_save_path();

        // Create a backup of current save.bin before overwriting
        if save_path.exists() {
            // Ensure backup directory exists
            fs::create_dir_all(&self.backup_dir).context("Failed to create backup directory")?;

            // Create timestamped emergency backup to prevent overwriting
            let now = Local::now();
            let emergency_filename = format!("emergency_before_restore_{}.bak", now.format("%Y-%m-%d_%H-%M-%S"));
            let emergency_backup = self.backup_dir.join(emergency_filename);
            fs::copy(&save_path, &emergency_backup).context("Failed to create emergency backup")?;
        }

        // Copy backup to save.bin
        fs::copy(backup_path, &save_path).context("Failed to restore backup")?;

        Ok(())
    }

    /// Gets the path to save.bin
    pub fn get_save_path(&self) -> PathBuf {
        self.save_dir.join("save.bin")
    }

    /// Checks if save.bin exists
    pub fn save_exists(&self) -> bool {
        self.get_save_path().exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().to_path_buf();
        let bm = BackupManager::new(save_dir.clone());

        // Create a test file
        let test_file = save_dir.join("save.bin");
        fs::write(&test_file, b"test data").unwrap();

        // Create backup
        let backup_path = bm.create_backup(&test_file).unwrap();

        assert!(backup_path.exists());
        assert_eq!(fs::read(backup_path).unwrap(), b"test data");
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().to_path_buf();
        let bm = BackupManager::new(save_dir.clone());

        // Create some backup files
        fs::write(save_dir.join("2024-01-01_12-00-00.bak"), b"backup1").unwrap();
        fs::write(save_dir.join("2024-01-02_12-00-00.bak"), b"backup2").unwrap();

        let backups = bm.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_restore_backup() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().to_path_buf();
        let bm = BackupManager::new(save_dir.clone());

        // Create backup
        let backup_path = save_dir.join("test.bak");
        fs::write(&backup_path, b"backup data").unwrap();

        // Create current save
        let save_path = bm.get_save_path();
        fs::write(&save_path, b"current data").unwrap();

        // Restore
        bm.restore_backup(&backup_path).unwrap();

        assert_eq!(fs::read(save_path).unwrap(), b"backup data");
    }
}
