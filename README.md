# 🏠 Hutch

Checkpoint and undo system for AI agent sessions - safe burrow to return to.

[![Crates.io](https://img.shields.io/crates/v/hutch.svg)](https://crates.io/crates/hutch)
[![Documentation](https://docs.rs/hutch/badge.svg)](https://docs.rs/hutch)
[![License](https://img.shields.io/crates/l/hutch.svg)](LICENSE)

## Overview

Hutch provides checkpoint and undo functionality for AI agent sessions, allowing you to save state and rollback changes.

## Features

- 💾 Automatic checkpoints at turn boundaries
- 📝 Named manual checkpoints
- ↩️ Undo to previous states
- 📁 File change tracking
- 💿 Persistent storage backend

## Installation

```toml
[dependencies]
hutch = "0.1"
```

## Usage

```rust
use hutch::{CheckpointManager, CheckpointConfig};

#[tokio::main]
async fn main() -> Result<(), hutch::CheckpointError> {
    let config = CheckpointConfig::default();
    let manager = CheckpointManager::new(config);

    // Save a named checkpoint
    let checkpoint_id = manager.save(Some("before refactor".into())).await?;
    println!("Saved checkpoint: {}", checkpoint_id);

    // Do some work...

    // Undo to the last checkpoint
    manager.undo().await?;

    // Or restore a specific checkpoint
    manager.restore(checkpoint_id).await?;

    // List all checkpoints
    for meta in manager.list() {
        println!("{}: {} ({})", meta.id, meta.summary, meta.timestamp);
    }

    Ok(())
}
```

## Auto-checkpointing

```rust
use hutch::{CheckpointManager, CheckpointConfig};
use warhorn::TaskId;

// Enable auto-checkpointing at turn boundaries
let config = CheckpointConfig {
    auto_checkpoint: true,
    max_checkpoints: 50,
    ..Default::default()
};

let manager = CheckpointManager::new(config);

// Checkpoint automatically saved at each turn
let task_id = TaskId::new();
manager.checkpoint_turn(task_id, 1).await?;
manager.checkpoint_turn(task_id, 2).await?;
```

## File Tracking

```rust
use std::path::PathBuf;

// Track file changes for checkpoint restore
manager.record_file_change(
    PathBuf::from("src/main.rs"),
    Some("old content".into()),
    "new content".into(),
);
```

## Part of the Goblin Family

- [warhorn](https://crates.io/crates/warhorn) - Protocol types
- [trinkets](https://crates.io/crates/trinkets) - Tool registry
- [wardstone](https://crates.io/crates/wardstone) - Sandboxing
- [skulk](https://crates.io/crates/skulk) - MCP connections
- **hutch** - Checkpoints (you are here)
- [ambush](https://crates.io/crates/ambush) - Task planning
- [cabal](https://crates.io/crates/cabal) - Orchestration

## License

MIT OR Apache-2.0
