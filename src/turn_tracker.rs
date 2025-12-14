//! Turn tracking for conversation history

use std::collections::HashMap;
use warhorn::CheckpointId;

/// Tracks turns and their associated checkpoints
pub struct TurnTracker {
    /// Turn number to checkpoint mapping
    turns: HashMap<u32, CheckpointId>,
    /// Current turn number
    current_turn: u32,
    /// Turn count
    count: u32,
}

impl TurnTracker {
    /// Create a new turn tracker
    pub fn new() -> Self {
        Self {
            turns: HashMap::new(),
            current_turn: 0,
            count: 0,
        }
    }

    /// Record a new turn with its checkpoint
    pub fn record_turn(&mut self, turn_number: u32, checkpoint_id: CheckpointId) {
        self.turns.insert(turn_number, checkpoint_id);
        self.current_turn = turn_number;
        self.count = self.count.max(turn_number + 1);
    }

    /// Get checkpoint for a turn
    pub fn checkpoint_for_turn(&self, turn_number: u32) -> Option<CheckpointId> {
        self.turns.get(&turn_number).copied()
    }

    /// Get current turn number
    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    /// Get total turn count
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Get the previous turn's checkpoint
    pub fn previous_checkpoint(&self) -> Option<CheckpointId> {
        if self.current_turn == 0 {
            None
        } else {
            self.checkpoint_for_turn(self.current_turn - 1)
        }
    }

    /// Get all turns with checkpoints
    pub fn all_turns(&self) -> Vec<(u32, CheckpointId)> {
        let mut turns: Vec<_> = self.turns.iter()
            .map(|(&turn, &id)| (turn, id))
            .collect();
        turns.sort_by_key(|(turn, _)| *turn);
        turns
    }

    /// Clear all turns
    pub fn clear(&mut self) {
        self.turns.clear();
        self.current_turn = 0;
        self.count = 0;
    }
}

impl Default for TurnTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_tracker() {
        let mut tracker = TurnTracker::new();
        
        let cp1 = CheckpointId::new();
        let cp2 = CheckpointId::new();
        
        tracker.record_turn(0, cp1);
        tracker.record_turn(1, cp2);
        
        assert_eq!(tracker.current_turn(), 1);
        assert_eq!(tracker.count(), 2);
        assert_eq!(tracker.checkpoint_for_turn(0), Some(cp1));
        assert_eq!(tracker.previous_checkpoint(), Some(cp1));
    }
}
