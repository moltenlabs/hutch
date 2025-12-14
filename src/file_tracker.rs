//! File change tracking

use std::collections::HashMap;
use std::path::PathBuf;

/// Tracks file changes for checkpoint/restore
pub struct FileTracker {
    /// Current file states
    states: HashMap<PathBuf, String>,
    /// Pending changes since last checkpoint
    pending_changes: Vec<FileChange>,
}

/// A tracked file change
#[derive(Debug, Clone)]
pub struct FileChange {
    /// File path
    pub path: PathBuf,
    /// Content before change (None if new file)
    pub old_content: Option<String>,
    /// Content after change
    pub new_content: String,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

impl FileTracker {
    /// Create a new file tracker
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            pending_changes: Vec::new(),
        }
    }

    /// Record a file change
    pub fn record_change(
        &mut self,
        path: PathBuf,
        old_content: Option<String>,
        new_content: String,
    ) {
        self.pending_changes.push(FileChange {
            path: path.clone(),
            old_content,
            new_content: new_content.clone(),
            timestamp: std::time::Instant::now(),
        });
        
        self.states.insert(path, new_content);
    }

    /// Get current file states
    pub fn current_states(&self) -> HashMap<PathBuf, String> {
        self.states.clone()
    }

    /// Get pending changes since last checkpoint
    pub fn pending_changes(&self) -> &[FileChange] {
        &self.pending_changes
    }

    /// Clear pending changes (called after checkpoint)
    pub fn clear_pending(&mut self) {
        self.pending_changes.clear();
    }

    /// Reset to a specific state
    pub fn reset_to(&mut self, states: &HashMap<PathBuf, String>) {
        self.states = states.clone();
        self.pending_changes.clear();
    }

    /// Get state of a specific file
    pub fn get_state(&self, path: &PathBuf) -> Option<&String> {
        self.states.get(path)
    }

    /// Check if a file is tracked
    pub fn is_tracked(&self, path: &PathBuf) -> bool {
        self.states.contains_key(path)
    }

    /// Get number of tracked files
    pub fn tracked_count(&self) -> usize {
        self.states.len()
    }

    /// Calculate diff between two states
    pub fn diff(
        old: &HashMap<PathBuf, String>,
        new: &HashMap<PathBuf, String>,
    ) -> FileDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        // Find added and modified
        for (path, new_content) in new {
            match old.get(path) {
                None => added.push(path.clone()),
                Some(old_content) if old_content != new_content => {
                    modified.push(path.clone());
                }
                _ => {}
            }
        }

        // Find deleted
        for path in old.keys() {
            if !new.contains_key(path) {
                deleted.push(path.clone());
            }
        }

        FileDiff { added, modified, deleted }
    }
}

impl Default for FileTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Diff between two file states
#[derive(Debug, Clone, Default)]
pub struct FileDiff {
    /// Files that were added
    pub added: Vec<PathBuf>,
    /// Files that were modified
    pub modified: Vec<PathBuf>,
    /// Files that were deleted
    pub deleted: Vec<PathBuf>,
}

impl FileDiff {
    /// Check if diff is empty
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Get total number of changed files
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === FileTracker Creation Tests ===

    #[test]
    fn test_file_tracker_new() {
        let tracker = FileTracker::new();
        assert_eq!(tracker.tracked_count(), 0);
        assert!(tracker.pending_changes().is_empty());
    }

    #[test]
    fn test_file_tracker_default() {
        let tracker: FileTracker = Default::default();
        assert_eq!(tracker.tracked_count(), 0);
    }

    // === Record Change Tests ===

    #[test]
    fn test_record_new_file() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(
            PathBuf::from("/test.txt"),
            None,
            "hello".to_string(),
        );
        
        assert!(tracker.is_tracked(&PathBuf::from("/test.txt")));
        assert_eq!(tracker.tracked_count(), 1);
    }

    #[test]
    fn test_record_modified_file() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(
            PathBuf::from("/test.txt"),
            Some("old content".to_string()),
            "new content".to_string(),
        );
        
        assert!(tracker.is_tracked(&PathBuf::from("/test.txt")));
        assert_eq!(tracker.get_state(&PathBuf::from("/test.txt")), Some(&"new content".to_string()));
    }

    #[test]
    fn test_record_multiple_changes() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/a.txt"), None, "a".to_string());
        tracker.record_change(PathBuf::from("/b.txt"), None, "b".to_string());
        tracker.record_change(PathBuf::from("/c.txt"), None, "c".to_string());
        
        assert_eq!(tracker.tracked_count(), 3);
        assert_eq!(tracker.pending_changes().len(), 3);
    }

    #[test]
    fn test_record_overwrite_same_file() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/test.txt"), None, "first".to_string());
        tracker.record_change(PathBuf::from("/test.txt"), Some("first".to_string()), "second".to_string());
        
        // Should still have 1 tracked file but 2 pending changes
        assert_eq!(tracker.tracked_count(), 1);
        assert_eq!(tracker.pending_changes().len(), 2);
        assert_eq!(tracker.get_state(&PathBuf::from("/test.txt")), Some(&"second".to_string()));
    }

    // === Pending Changes Tests ===

    #[test]
    fn test_pending_changes() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/a.txt"), None, "content".to_string());
        
        let changes = tracker.pending_changes();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("/a.txt"));
        assert!(changes[0].old_content.is_none());
        assert_eq!(changes[0].new_content, "content");
    }

    #[test]
    fn test_clear_pending() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/a.txt"), None, "content".to_string());
        assert!(!tracker.pending_changes().is_empty());
        
        tracker.clear_pending();
        assert!(tracker.pending_changes().is_empty());
        
        // File should still be tracked
        assert!(tracker.is_tracked(&PathBuf::from("/a.txt")));
    }

    // === State Tests ===

    #[test]
    fn test_current_states() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/a.txt"), None, "a".to_string());
        tracker.record_change(PathBuf::from("/b.txt"), None, "b".to_string());
        
        let states = tracker.current_states();
        assert_eq!(states.len(), 2);
        assert_eq!(states.get(&PathBuf::from("/a.txt")), Some(&"a".to_string()));
        assert_eq!(states.get(&PathBuf::from("/b.txt")), Some(&"b".to_string()));
    }

    #[test]
    fn test_get_state() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/test.txt"), None, "content".to_string());
        
        assert_eq!(tracker.get_state(&PathBuf::from("/test.txt")), Some(&"content".to_string()));
        assert!(tracker.get_state(&PathBuf::from("/nonexistent.txt")).is_none());
    }

    #[test]
    fn test_is_tracked() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/test.txt"), None, "content".to_string());
        
        assert!(tracker.is_tracked(&PathBuf::from("/test.txt")));
        assert!(!tracker.is_tracked(&PathBuf::from("/other.txt")));
    }

    // === Reset Tests ===

    #[test]
    fn test_reset_to() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/a.txt"), None, "a".to_string());
        tracker.record_change(PathBuf::from("/b.txt"), None, "b".to_string());
        
        let mut new_states = HashMap::new();
        new_states.insert(PathBuf::from("/c.txt"), "c".to_string());
        
        tracker.reset_to(&new_states);
        
        assert_eq!(tracker.tracked_count(), 1);
        assert!(tracker.is_tracked(&PathBuf::from("/c.txt")));
        assert!(!tracker.is_tracked(&PathBuf::from("/a.txt")));
        assert!(tracker.pending_changes().is_empty());
    }

    // === Diff Tests ===

    #[test]
    fn test_diff_basic() {
        let mut old = HashMap::new();
        old.insert(PathBuf::from("/a.txt"), "old".to_string());
        old.insert(PathBuf::from("/b.txt"), "unchanged".to_string());
        old.insert(PathBuf::from("/c.txt"), "deleted".to_string());
        
        let mut new = HashMap::new();
        new.insert(PathBuf::from("/a.txt"), "new".to_string());
        new.insert(PathBuf::from("/b.txt"), "unchanged".to_string());
        new.insert(PathBuf::from("/d.txt"), "added".to_string());
        
        let diff = FileTracker::diff(&old, &new);
        
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.deleted.len(), 1);
        
        assert!(diff.added.contains(&PathBuf::from("/d.txt")));
        assert!(diff.modified.contains(&PathBuf::from("/a.txt")));
        assert!(diff.deleted.contains(&PathBuf::from("/c.txt")));
    }

    #[test]
    fn test_diff_empty_to_nonempty() {
        let old = HashMap::new();
        let mut new = HashMap::new();
        new.insert(PathBuf::from("/a.txt"), "a".to_string());
        new.insert(PathBuf::from("/b.txt"), "b".to_string());
        
        let diff = FileTracker::diff(&old, &new);
        
        assert_eq!(diff.added.len(), 2);
        assert!(diff.modified.is_empty());
        assert!(diff.deleted.is_empty());
    }

    #[test]
    fn test_diff_nonempty_to_empty() {
        let mut old = HashMap::new();
        old.insert(PathBuf::from("/a.txt"), "a".to_string());
        old.insert(PathBuf::from("/b.txt"), "b".to_string());
        let new = HashMap::new();
        
        let diff = FileTracker::diff(&old, &new);
        
        assert!(diff.added.is_empty());
        assert!(diff.modified.is_empty());
        assert_eq!(diff.deleted.len(), 2);
    }

    #[test]
    fn test_diff_no_changes() {
        let mut old = HashMap::new();
        old.insert(PathBuf::from("/a.txt"), "a".to_string());
        
        let new = old.clone();
        
        let diff = FileTracker::diff(&old, &new);
        
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    // === FileDiff Tests ===

    #[test]
    fn test_file_diff_default() {
        let diff: FileDiff = Default::default();
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn test_file_diff_is_empty() {
        let diff = FileDiff {
            added: vec![],
            modified: vec![],
            deleted: vec![],
        };
        assert!(diff.is_empty());
    }

    #[test]
    fn test_file_diff_not_empty_added() {
        let diff = FileDiff {
            added: vec![PathBuf::from("/a.txt")],
            modified: vec![],
            deleted: vec![],
        };
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_file_diff_not_empty_modified() {
        let diff = FileDiff {
            added: vec![],
            modified: vec![PathBuf::from("/a.txt")],
            deleted: vec![],
        };
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_file_diff_not_empty_deleted() {
        let diff = FileDiff {
            added: vec![],
            modified: vec![],
            deleted: vec![PathBuf::from("/a.txt")],
        };
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_file_diff_total_changes() {
        let diff = FileDiff {
            added: vec![PathBuf::from("/a.txt"), PathBuf::from("/b.txt")],
            modified: vec![PathBuf::from("/c.txt")],
            deleted: vec![PathBuf::from("/d.txt"), PathBuf::from("/e.txt"), PathBuf::from("/f.txt")],
        };
        assert_eq!(diff.total_changes(), 6);
    }

    // === FileChange Tests ===

    #[test]
    fn test_file_change_new_file() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(PathBuf::from("/new.txt"), None, "content".to_string());
        
        let change = &tracker.pending_changes()[0];
        assert!(change.old_content.is_none());
        assert_eq!(change.new_content, "content");
    }

    #[test]
    fn test_file_change_modified_file() {
        let mut tracker = FileTracker::new();
        
        tracker.record_change(
            PathBuf::from("/mod.txt"),
            Some("old".to_string()),
            "new".to_string(),
        );
        
        let change = &tracker.pending_changes()[0];
        assert_eq!(change.old_content, Some("old".to_string()));
        assert_eq!(change.new_content, "new");
    }
}
