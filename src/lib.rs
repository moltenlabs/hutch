//! # Hutch
//!
//! Checkpoint and undo system for AI agent sessions - safe burrow to return to.
//!
//! This crate provides:
//! - Automatic checkpoints at each turn
//! - Named manual checkpoints
//! - Undo/restore functionality
//! - File diff tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │                 CheckpointManager                │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐ │
//! │  │ Checkpoint │  │ Checkpoint │  │ Checkpoint │ │
//! │  │    (T0)    │  │    (T1)    │  │    (T2)    │ │
//! │  └────────────┘  └────────────┘  └────────────┘ │
//! │       │              │              │           │
//! │       ▼              ▼              ▼           │
//! │  ┌────────────────────────────────────────────┐ │
//! │  │              TurnTracker                   │ │
//! │  │   [Turn 0] → [Turn 1] → [Turn 2] → ...    │ │
//! │  └────────────────────────────────────────────┘ │
//! │                      │                          │
//! │                      ▼                          │
//! │  ┌────────────────────────────────────────────┐ │
//! │  │              FileTracker                   │ │
//! │  │   File diffs, snapshots, restore           │ │
//! │  └────────────────────────────────────────────┘ │
//! └──────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use hutch::{CheckpointManager, CheckpointConfig};
//!
//! let config = CheckpointConfig::default();
//! let manager = CheckpointManager::new(config);
//!
//! // Auto-checkpoint at turn boundary
//! let checkpoint_id = manager.checkpoint_turn(turn_number).await?;
//!
//! // Manual checkpoint
//! let checkpoint_id = manager.save("before refactor").await?;
//!
//! // Undo to last checkpoint
//! manager.undo().await?;
//!
//! // Restore specific checkpoint
//! manager.restore(checkpoint_id).await?;
//! ```

pub mod manager;
pub mod checkpoint;
pub mod turn_tracker;
pub mod file_tracker;
pub mod storage;
pub mod error;

pub use manager::CheckpointManager;
pub use checkpoint::{Checkpoint, CheckpointData};
pub use turn_tracker::TurnTracker;
pub use file_tracker::FileTracker;
pub use error::CheckpointError;

// Re-export protocol types
pub use warhorn::{CheckpointId, CheckpointMeta};
