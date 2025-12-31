//! Undo/redo history

/// Simple undo/redo history
#[derive(Debug)]
pub struct History<T> {
    /// Past states (for undo)
    undo_stack: Vec<T>,
    /// Future states (for redo)
    redo_stack: Vec<T>,
    /// Maximum history size
    max_size: usize,
}

impl<T: Clone> History<T> {
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Push a new state (clears redo stack)
    pub fn push(&mut self, state: T) {
        self.undo_stack.push(state);
        self.redo_stack.clear();

        // Trim if over max size
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: pop from undo stack
    pub fn undo(&mut self) -> Option<T> {
        self.undo_stack.pop()
    }

    /// Redo: pop from redo stack
    pub fn redo(&mut self) -> Option<T> {
        self.redo_stack.pop()
    }

    /// Save current state before undo for potential redo
    pub fn save_for_redo(&mut self, state: T) {
        self.redo_stack.push(state);
    }

    /// Check if undo is available
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_push_and_undo() {
        let mut history: History<i32> = History::new(10);

        history.push(1);
        history.push(2);
        history.push(3);

        assert!(history.can_undo());
        assert!(!history.can_redo());

        assert_eq!(history.undo(), Some(3));
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn test_history_max_size() {
        let mut history: History<i32> = History::new(3);

        history.push(1);
        history.push(2);
        history.push(3);
        history.push(4);

        // Should have dropped the oldest
        assert_eq!(history.undo(), Some(4));
        assert_eq!(history.undo(), Some(3));
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn test_history_clear() {
        let mut history: History<i32> = History::new(10);

        history.push(1);
        history.push(2);

        history.clear();

        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
