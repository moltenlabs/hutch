//! Checkpoint error types

use thiserror::Error;
use warhorn::CheckpointId;

/// Errors that can occur in checkpoint operations
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Checkpoint not found
    #[error("Checkpoint not found: {0}")]
    NotFound(CheckpointId),

    /// Nothing to undo
    #[error("Nothing to undo")]
    NothingToUndo,

    /// Auto-checkpoint is disabled
    #[error("Auto-checkpoint is disabled")]
    AutoCheckpointDisabled,

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Restore error
    #[error("Restore error: {0}")]
    RestoreError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
