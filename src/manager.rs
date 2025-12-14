//! Checkpoint manager - coordinates checkpointing

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use warhorn::{CheckpointId, CheckpointMeta, TaskId};
use crate::checkpoint::{Checkpoint, CheckpointData};
use crate::turn_tracker::TurnTracker;
use crate::file_tracker::FileTracker;
use crate::error::CheckpointError;

/// Configuration for checkpoint manager
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Storage directory for checkpoints
    pub storage_dir: PathBuf,
    /// Maximum number of checkpoints to retain
    pub max_checkpoints: usize,
    /// Auto-checkpoint on each turn
    pub auto_checkpoint: bool,
    /// Track file changes
    pub track_files: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from(".lair/checkpoints"),
            max_checkpoints: 50,
            auto_checkpoint: true,
            track_files: true,
        }
    }
}

/// Manages checkpoints and undo functionality
pub struct CheckpointManager {
    /// Configuration
    config: CheckpointConfig,
    /// All checkpoints
    checkpoints: RwLock<HashMap<CheckpointId, Checkpoint>>,
    /// Checkpoint order (oldest first)
    order: RwLock<Vec<CheckpointId>>,
    /// Turn tracker
    turn_tracker: RwLock<TurnTracker>,
    /// File tracker
    file_tracker: RwLock<FileTracker>,
    /// Current checkpoint (for undo)
    current: RwLock<Option<CheckpointId>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(config: CheckpointConfig) -> Self {
        Self {
            config,
            checkpoints: RwLock::new(HashMap::new()),
            order: RwLock::new(Vec::new()),
            turn_tracker: RwLock::new(TurnTracker::new()),
            file_tracker: RwLock::new(FileTracker::new()),
            current: RwLock::new(None),
        }
    }

    /// Save a manual checkpoint
    pub async fn save(&self, name: Option<String>) -> Result<CheckpointId, CheckpointError> {
        let id = CheckpointId::new();
        let timestamp = Utc::now();
        
        // Collect checkpoint data
        let data = self.collect_checkpoint_data().await?;
        
        let checkpoint = Checkpoint {
            id,
            name: name.clone(),
            timestamp,
            task_id: None,
            turn_number: None,
            data,
        };

        let size = self.store_checkpoint(&checkpoint).await?;
        
        // Add to registry
        self.checkpoints.write().insert(id, checkpoint);
        self.order.write().push(id);
        *self.current.write() = Some(id);

        // Prune old checkpoints
        self.prune_checkpoints().await?;

        info!(
            checkpoint_id = %id,
            name = ?name,
            size = size,
            "Saved checkpoint"
        );

        Ok(id)
    }

    /// Checkpoint at turn boundary (auto-checkpoint)
    pub async fn checkpoint_turn(
        &self,
        task_id: TaskId,
        turn_number: u32,
    ) -> Result<CheckpointId, CheckpointError> {
        if !self.config.auto_checkpoint {
            return Err(CheckpointError::AutoCheckpointDisabled);
        }

        let id = CheckpointId::new();
        let timestamp = Utc::now();
        
        // Collect checkpoint data
        let data = self.collect_checkpoint_data().await?;
        
        let checkpoint = Checkpoint {
            id,
            name: None,
            timestamp,
            task_id: Some(task_id),
            turn_number: Some(turn_number),
            data,
        };

        let size = self.store_checkpoint(&checkpoint).await?;
        
        // Add to registry
        self.checkpoints.write().insert(id, checkpoint);
        self.order.write().push(id);
        *self.current.write() = Some(id);

        // Update turn tracker
        self.turn_tracker.write().record_turn(turn_number, id);

        // Prune old checkpoints
        self.prune_checkpoints().await?;

        debug!(
            checkpoint_id = %id,
            task_id = %task_id,
            turn = turn_number,
            "Saved turn checkpoint"
        );

        Ok(id)
    }

    /// Undo to the last checkpoint
    pub async fn undo(&self) -> Result<CheckpointId, CheckpointError> {
        let current = self.current.read().clone();
        
        // Find the previous checkpoint
        let order = self.order.read();
        let current_idx = current
            .and_then(|id| order.iter().position(|&i| i == id))
            .unwrap_or(order.len());
        
        if current_idx == 0 {
            return Err(CheckpointError::NothingToUndo);
        }

        let target_id = order[current_idx - 1];
        drop(order);

        self.restore(target_id).await
    }

    /// Restore a specific checkpoint
    pub async fn restore(&self, checkpoint_id: CheckpointId) -> Result<CheckpointId, CheckpointError> {
        let checkpoint = self.checkpoints.read()
            .get(&checkpoint_id)
            .cloned()
            .ok_or(CheckpointError::NotFound(checkpoint_id))?;

        info!(
            checkpoint_id = %checkpoint_id,
            name = ?checkpoint.name,
            "Restoring checkpoint"
        );

        // Restore file state
        self.restore_files(&checkpoint.data).await?;

        // Update current
        *self.current.write() = Some(checkpoint_id);

        Ok(checkpoint_id)
    }

    /// List all checkpoints
    pub fn list(&self) -> Vec<CheckpointMeta> {
        let checkpoints = self.checkpoints.read();
        let order = self.order.read();
        
        order.iter()
            .filter_map(|id| checkpoints.get(id).map(|c| c.to_meta()))
            .collect()
    }

    /// Get a checkpoint by ID
    pub fn get(&self, id: &CheckpointId) -> Option<Checkpoint> {
        self.checkpoints.read().get(id).cloned()
    }

    /// Get current checkpoint ID
    pub fn current(&self) -> Option<CheckpointId> {
        *self.current.read()
    }

    /// Get checkpoint count
    pub fn count(&self) -> usize {
        self.checkpoints.read().len()
    }

    /// Record a file change for tracking
    pub fn record_file_change(&self, path: PathBuf, old_content: Option<String>, new_content: String) {
        self.file_tracker.write().record_change(path, old_content, new_content);
    }

    // === Private Methods ===

    async fn collect_checkpoint_data(&self) -> Result<CheckpointData, CheckpointError> {
        let file_tracker = self.file_tracker.read();
        
        Ok(CheckpointData {
            file_states: file_tracker.current_states(),
            conversation_snapshot: None, // TODO: Implement
            agent_states: HashMap::new(), // TODO: Implement
        })
    }

    async fn store_checkpoint(&self, checkpoint: &Checkpoint) -> Result<u64, CheckpointError> {
        // In a full implementation, this would persist to disk
        // For now, just calculate approximate size
        let json = serde_json::to_string(&checkpoint)
            .map_err(|e| CheckpointError::StorageError(e.to_string()))?;
        
        Ok(json.len() as u64)
    }

    async fn restore_files(&self, data: &CheckpointData) -> Result<(), CheckpointError> {
        for (path, content) in &data.file_states {
            tokio::fs::write(path, content).await
                .map_err(|e| CheckpointError::RestoreError(format!(
                    "Failed to restore {}: {}", path.display(), e
                )))?;
        }
        
        // Update file tracker
        self.file_tracker.write().reset_to(&data.file_states);
        
        Ok(())
    }

    async fn prune_checkpoints(&self) -> Result<(), CheckpointError> {
        let mut order = self.order.write();
        let mut checkpoints = self.checkpoints.write();
        
        while order.len() > self.config.max_checkpoints {
            if let Some(oldest_id) = order.first().copied() {
                // Don't remove named checkpoints
                if let Some(checkpoint) = checkpoints.get(&oldest_id) {
                    if checkpoint.name.is_some() {
                        // Skip named checkpoints
                        order.remove(0);
                        continue;
                    }
                }
                
                order.remove(0);
                checkpoints.remove(&oldest_id);
                debug!(checkpoint_id = %oldest_id, "Pruned old checkpoint");
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let config = CheckpointConfig::default();
        let manager = CheckpointManager::new(config);
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_save_checkpoint() {
        let config = CheckpointConfig::default();
        let manager = CheckpointManager::new(config);
        
        let id = manager.save(Some("test".to_string())).await.unwrap();
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.current(), Some(id));
    }

    #[tokio::test]
    async fn test_list_checkpoints() {
        let config = CheckpointConfig::default();
        let manager = CheckpointManager::new(config);
        
        manager.save(Some("first".to_string())).await.unwrap();
        manager.save(Some("second".to_string())).await.unwrap();
        
        let list = manager.list();
        assert_eq!(list.len(), 2);
    }
}
