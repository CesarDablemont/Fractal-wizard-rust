pub struct UndoStack<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    max: usize,
}

impl<T> UndoStack<T> {
    pub fn new(max: usize) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), max }
    }

    pub fn push(&mut self, state: T) {
        self.undo.push(state);
        self.redo.clear();
        if self.undo.len() > self.max {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
