//! Checkpoint data structures

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use warhorn::{CheckpointId, CheckpointMeta, TaskId};

/// A single checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier
    pub id: CheckpointId,
    /// Optional name (for manual checkpoints)
    pub name: Option<String>,
    /// When checkpoint was created
    pub timestamp: DateTime<Utc>,
    /// Associated task ID
    pub task_id: Option<TaskId>,
    /// Turn number (for auto-checkpoints)
    pub turn_number: Option<u32>,
    /// Checkpoint data
    pub data: CheckpointData,
}

impl Checkpoint {
    /// Convert to protocol metadata
    pub fn to_meta(&self) -> CheckpointMeta {
        CheckpointMeta {
            id: self.id,
            name: self.name.clone(),
            timestamp: self.timestamp,
            size_bytes: self.estimated_size(),
            task_id: self.task_id,
            summary: self.summary(),
        }
    }

    /// Get summary description
    pub fn summary(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else if let Some(turn) = self.turn_number {
            format!("Turn {}", turn)
        } else {
            format!("Checkpoint at {}", self.timestamp.format("%H:%M:%S"))
        }
    }

    /// Estimate size in bytes
    pub fn estimated_size(&self) -> u64 {
        // Simple estimate based on file content sizes
        self.data.file_states.values()
            .map(|s| s.len() as u64)
            .sum()
    }
}

/// Data stored in a checkpoint
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointData {
    /// File contents at checkpoint time
    pub file_states: HashMap<PathBuf, String>,
    /// Conversation history snapshot
    pub conversation_snapshot: Option<ConversationSnapshot>,
    /// Agent state snapshots
    pub agent_states: HashMap<String, serde_json::Value>,
}

/// Snapshot of conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSnapshot {
    /// Messages in the conversation
    pub messages: Vec<ConversationMessage>,
    /// Total token count
    pub token_count: u64,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Summary Tests ===

    #[test]
    fn test_checkpoint_summary_named() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: Some("My checkpoint".to_string()),
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data: CheckpointData::default(),
        };
        
        assert_eq!(checkpoint.summary(), "My checkpoint");
    }

    #[test]
    fn test_checkpoint_turn_summary() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: Some(5),
            data: CheckpointData::default(),
        };
        
        assert_eq!(checkpoint.summary(), "Turn 5");
    }

    #[test]
    fn test_checkpoint_timestamp_summary() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data: CheckpointData::default(),
        };
        
        assert!(checkpoint.summary().contains("Checkpoint at"));
    }

    #[test]
    fn test_checkpoint_name_priority_over_turn() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: Some("Named".to_string()),
            timestamp: Utc::now(),
            task_id: None,
            turn_number: Some(5),
            data: CheckpointData::default(),
        };
        
        // Name should take priority
        assert_eq!(checkpoint.summary(), "Named");
    }

    // === Size Estimation Tests ===

    #[test]
    fn test_checkpoint_size_empty() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data: CheckpointData::default(),
        };
        
        assert_eq!(checkpoint.estimated_size(), 0);
    }

    #[test]
    fn test_checkpoint_size_with_files() {
        let mut data = CheckpointData::default();
        data.file_states.insert(PathBuf::from("/a.txt"), "hello".to_string());
        data.file_states.insert(PathBuf::from("/b.txt"), "world".to_string());
        
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data,
        };
        
        assert_eq!(checkpoint.estimated_size(), 10); // 5 + 5
    }

    #[test]
    fn test_checkpoint_size_large_files() {
        let mut data = CheckpointData::default();
        let large_content = "x".repeat(10_000);
        data.file_states.insert(PathBuf::from("/large.txt"), large_content);
        
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: None,
            data,
        };
        
        assert_eq!(checkpoint.estimated_size(), 10_000);
    }

    // === to_meta Tests ===

    #[test]
    fn test_checkpoint_to_meta() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: Some("Test".to_string()),
            timestamp: Utc::now(),
            task_id: Some(TaskId::new()),
            turn_number: Some(3),
            data: CheckpointData::default(),
        };
        
        let meta = checkpoint.to_meta();
        
        assert_eq!(meta.id, checkpoint.id);
        assert_eq!(meta.name, Some("Test".to_string()));
        assert_eq!(meta.timestamp, checkpoint.timestamp);
        assert_eq!(meta.size_bytes, 0);
        assert!(meta.task_id.is_some());
        assert_eq!(meta.summary, "Test");
    }

    #[test]
    fn test_checkpoint_to_meta_with_size() {
        let mut data = CheckpointData::default();
        data.file_states.insert(PathBuf::from("/test.txt"), "content".to_string());
        
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: None,
            timestamp: Utc::now(),
            task_id: None,
            turn_number: Some(1),
            data,
        };
        
        let meta = checkpoint.to_meta();
        assert_eq!(meta.size_bytes, 7);
        assert_eq!(meta.summary, "Turn 1");
    }

    // === CheckpointData Tests ===

    #[test]
    fn test_checkpoint_data_default() {
        let data: CheckpointData = Default::default();
        
        assert!(data.file_states.is_empty());
        assert!(data.conversation_snapshot.is_none());
        assert!(data.agent_states.is_empty());
    }

    #[test]
    fn test_checkpoint_data_with_files() {
        let mut data = CheckpointData::default();
        data.file_states.insert(PathBuf::from("/a.rs"), "fn main() {}".to_string());
        data.file_states.insert(PathBuf::from("/b.rs"), "mod test;".to_string());
        
        assert_eq!(data.file_states.len(), 2);
    }

    #[test]
    fn test_checkpoint_data_with_conversation() {
        let data = CheckpointData {
            file_states: HashMap::new(),
            conversation_snapshot: Some(ConversationSnapshot {
                messages: vec![
                    ConversationMessage {
                        role: "user".to_string(),
                        content: "Hello".to_string(),
                        timestamp: Utc::now(),
                    },
                    ConversationMessage {
                        role: "assistant".to_string(),
                        content: "Hi there!".to_string(),
                        timestamp: Utc::now(),
                    },
                ],
                token_count: 100,
            }),
            agent_states: HashMap::new(),
        };
        
        let snapshot = data.conversation_snapshot.unwrap();
        assert_eq!(snapshot.messages.len(), 2);
        assert_eq!(snapshot.token_count, 100);
    }

    #[test]
    fn test_checkpoint_data_with_agent_states() {
        let mut data = CheckpointData::default();
        data.agent_states.insert(
            "agent-1".to_string(),
            serde_json::json!({"status": "running", "progress": 50}),
        );
        
        assert_eq!(data.agent_states.len(), 1);
    }

    // === ConversationSnapshot Tests ===

    #[test]
    fn test_conversation_snapshot() {
        let snapshot = ConversationSnapshot {
            messages: vec![
                ConversationMessage {
                    role: "system".to_string(),
                    content: "You are an assistant".to_string(),
                    timestamp: Utc::now(),
                },
            ],
            token_count: 50,
        };
        
        assert_eq!(snapshot.messages.len(), 1);
        assert_eq!(snapshot.token_count, 50);
    }

    // === ConversationMessage Tests ===

    #[test]
    fn test_conversation_message_roles() {
        let roles = vec!["user", "assistant", "system", "tool"];
        
        for role in roles {
            let msg = ConversationMessage {
                role: role.to_string(),
                content: "test".to_string(),
                timestamp: Utc::now(),
            };
            assert_eq!(msg.role, role);
        }
    }

    // === Serialization Tests ===

    #[test]
    fn test_checkpoint_serialization() {
        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: Some("Test".to_string()),
            timestamp: Utc::now(),
            task_id: None,
            turn_number: Some(1),
            data: CheckpointData::default(),
        };
        
        let json = serde_json::to_string(&checkpoint).unwrap();
        let parsed: Checkpoint = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.id, checkpoint.id);
        assert_eq!(parsed.name, checkpoint.name);
        assert_eq!(parsed.turn_number, checkpoint.turn_number);
    }

    #[test]
    fn test_checkpoint_data_serialization() {
        let mut data = CheckpointData::default();
        data.file_states.insert(PathBuf::from("/test.txt"), "content".to_string());
        
        let json = serde_json::to_string(&data).unwrap();
        let parsed: CheckpointData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.file_states.len(), 1);
    }

    #[test]
    fn test_conversation_snapshot_serialization() {
        let snapshot = ConversationSnapshot {
            messages: vec![
                ConversationMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                    timestamp: Utc::now(),
                },
            ],
            token_count: 10,
        };
        
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: ConversationSnapshot = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.token_count, 10);
    }
}
