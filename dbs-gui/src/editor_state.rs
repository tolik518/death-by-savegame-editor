use crate::backup::BackupInfo;
use crate::hocon_parser::HoconDocument;
use std::path::PathBuf;

#[derive(Clone)]
pub enum EditorState {
    /// Initial welcome screen with main options
    Welcome,

    /// Loading save file in progress
    LoadingSave,

    /// Active editing session (raw text mode)
    Editing {
        original_path: PathBuf,
        backup_path: PathBuf,
        content: String,
        is_modified: bool,
        original_content: String, // Preserve original for true raw mode
    },

    /// Active editing session (parsed form mode)
    EditingParsed {
        original_path: PathBuf,
        backup_path: PathBuf,
        document: HoconDocument,
        is_modified: bool,
        edit_mode: EditMode,
    },

    /// Showing list of available backups
    SelectingBackup { backups: Vec<BackupInfo> },

    /// Confirming restore operation
    ConfirmRestore { backup_info: BackupInfo },

    /// Error state with optional retry
    Error {
        message: String,
        previous_state: Box<EditorState>,
    },
}

#[derive(Clone, PartialEq)]
pub enum EditMode {
    Form, // User-friendly form interface
    Raw,  // Raw text editor
}

impl Default for EditorState {
    fn default() -> Self {
        Self::Welcome
    }
}

pub enum EditorAction {
    // Welcome screen actions
    LoadSavegame,
    RecoverBackup,

    // File selection
    FileSelected(PathBuf),
    FileCancelled,

    // Editing actions
    EditContent(String),
    SaveChanges,
    CancelEdit,

    // Backup actions
    SelectBackup(BackupInfo),
    ConfirmRestore,
    CancelRestore,

    // Error handling
    DismissError,
    RetryLastAction,
}
