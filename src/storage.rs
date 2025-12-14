//! Checkpoint storage backend

use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};

use warhorn::CheckpointId;
use crate::checkpoint::Checkpoint;
use crate::error::CheckpointError;

/// Storage backend for checkpoints
pub struct CheckpointStorage {
    /// Base directory for checkpoint storage
    base_dir: PathBuf,
}

impl CheckpointStorage {
    /// Create a new storage backend
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Initialize storage directory
    pub async fn init(&self) -> Result<(), CheckpointError> {
        fs::create_dir_all(&self.base_dir).await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to create storage directory: {}", e
            )))?;
        
        debug!(dir = %self.base_dir.display(), "Initialized checkpoint storage");
        Ok(())
    }

    /// Save a checkpoint to storage
    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<u64, CheckpointError> {
        let path = self.checkpoint_path(&checkpoint.id);
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| CheckpointError::StorageError(format!(
                    "Failed to create directory: {}", e
                )))?;
        }
        
        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to serialize checkpoint: {}", e
            )))?;
        
        let size = json.len() as u64;
        
        fs::write(&path, &json).await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to write checkpoint: {}", e
            )))?;
        
        debug!(
            checkpoint_id = %checkpoint.id,
            path = %path.display(),
            size = size,
            "Saved checkpoint to storage"
        );
        
        Ok(size)
    }

    /// Load a checkpoint from storage
    pub async fn load(&self, id: &CheckpointId) -> Result<Checkpoint, CheckpointError> {
        let path = self.checkpoint_path(id);
        
        let json = fs::read_to_string(&path).await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to read checkpoint: {}", e
            )))?;
        
        let checkpoint: Checkpoint = serde_json::from_str(&json)
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to deserialize checkpoint: {}", e
            )))?;
        
        debug!(checkpoint_id = %id, "Loaded checkpoint from storage");
        Ok(checkpoint)
    }

    /// Delete a checkpoint from storage
    pub async fn delete(&self, id: &CheckpointId) -> Result<(), CheckpointError> {
        let path = self.checkpoint_path(id);
        
        if path.exists() {
            fs::remove_file(&path).await
                .map_err(|e| CheckpointError::StorageError(format!(
                    "Failed to delete checkpoint: {}", e
                )))?;
        }
        
        debug!(checkpoint_id = %id, "Deleted checkpoint from storage");
        Ok(())
    }

    /// Check if a checkpoint exists in storage
    pub async fn exists(&self, id: &CheckpointId) -> bool {
        self.checkpoint_path(id).exists()
    }

    /// List all checkpoint IDs in storage
    pub async fn list_ids(&self) -> Result<Vec<CheckpointId>, CheckpointError> {
        let mut ids = Vec::new();
        
        let mut entries = fs::read_dir(&self.base_dir).await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to read storage directory: {}", e
            )))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to read entry: {}", e
            )))?
        {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    if let Ok(uuid) = stem.to_string_lossy().parse::<uuid::Uuid>() {
                        ids.push(CheckpointId::from_uuid(uuid));
                    }
                }
            }
        }
        
        Ok(ids)
    }

    /// Get total storage size
    pub async fn total_size(&self) -> Result<u64, CheckpointError> {
        let mut total = 0u64;
        
        let mut entries = fs::read_dir(&self.base_dir).await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to read storage directory: {}", e
            )))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| CheckpointError::StorageError(format!(
                "Failed to read entry: {}", e
            )))?
        {
            if let Ok(metadata) = entry.metadata().await {
                total += metadata.len();
            }
        }
        
        Ok(total)
    }

    // Private helper
    fn checkpoint_path(&self, id: &CheckpointId) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::checkpoint::CheckpointData;
    use chrono::Utc;

    #[tokio::test]
    async fn test_storage_save_load() {
        let dir = tempdir().unwrap();
        let storage = CheckpointStorage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: Some("test".to_string()),
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data: CheckpointData::default(),
        };
        
        storage.save(&checkpoint).await.unwrap();
        
        let loaded = storage.load(&checkpoint.id).await.unwrap();
        assert_eq!(loaded.name, checkpoint.name);
    }
}
