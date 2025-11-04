use std::path::PathBuf;

/// Service for locating game save directories across platforms
pub struct SaveLocator;

impl SaveLocator {
    pub fn new() -> Self {
        Self
    }

    /// Get the platform-specific save directory
    ///
    /// Returns:
    /// - Linux: ~/.local/share/Terrible Toybox/Death by Scrolling
    /// - Windows: %LOCALAPPDATA%\Terrible Toybox\Death by Scrolling
    /// - macOS: ~/Library/Application Support/Terrible Toybox/Death by Scrolling
    pub fn get_save_directory(&self) -> Option<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;

        let mut path = PathBuf::from(home);

        if cfg!(target_os = "linux") {
            path.push(".local");
            path.push("share");
        } else if cfg!(target_os = "windows") {
            path.push("AppData");
            path.push("Local");
        } else if cfg!(target_os = "macos") {
            path.push("Library");
            path.push("Application Support");
        } else {
            return None;
        }

        path.push("Terrible Toybox");
        path.push("Death by Scrolling");

        Some(path)
    }

    /// Get the path to save.bin
    pub fn get_save_path(&self) -> Option<PathBuf> {
        let mut path = self.get_save_directory()?;
        path.push("save.bin");
        Some(path)
    }

    /// Check if save.bin exists
    pub fn save_exists(&self) -> bool {
        self.get_save_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}

impl Default for SaveLocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_save_directory_returns_some() {
        let locator = SaveLocator::new();
        let dir = locator.get_save_directory();
        // Should return Some on any supported platform
        assert!(dir.is_some() || cfg!(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos"
        ))));
    }

    #[test]
    fn test_save_path_includes_save_bin() {
        let locator = SaveLocator::new();
        if let Some(path) = locator.get_save_path() {
            assert!(path.to_string_lossy().contains("save.bin"));
        }
    }
}

