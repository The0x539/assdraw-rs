use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct UndoStack<T> {
    history: Vec<T>,
    active: Option<T>,
    idx: usize,
}

impl<T> UndoStack<T> {
    pub fn new(initial_state: T) -> Self {
        Self {
            history: vec![initial_state],
            active: None,
            idx: 0,
        }
    }

    #[inline]
    pub fn can_undo(&self) -> bool {
        self.idx > 0
    }

    pub fn undo(&mut self) {
        if self.can_undo() {
            self.idx -= 1;
            self.active = None;
        }
    }

    #[inline]
    pub fn can_redo(&self) -> bool {
        self.idx < self.history.len() - 1
    }

    pub fn redo(&mut self) {
        if self.can_redo() {
            self.idx += 1;
            self.active = None;
        }
    }

    pub fn push_state(&mut self, new_state: T) {
        self.active = None;
        self.history.truncate(self.idx + 1);
        self.history.push(new_state);
        self.idx += 1;
    }

    pub fn commit(&mut self) {
        if let Some(new_state) = self.active.take() {
            self.push_state(new_state);
        }
    }
}

impl<T: Default> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Deref for UndoStack<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.active.as_ref().unwrap_or(&self.history[self.idx])
    }
}

impl<T: Clone> DerefMut for UndoStack<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let history = &self.history;
        let idx = self.idx;
        self.active.get_or_insert_with(|| history[idx].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::UndoStack;

    #[test]
    fn test_undo_stack() {
        #[derive(Clone)]
        struct Foo {
            a: i32,
        }

        let mut s = UndoStack::new(Foo { a: 3 });
        s.a += 5;
        s.commit();

        s.a += 1;
        s.a += 2;
        s.commit();
        assert_eq!(s.a, 11);

        s.undo();
        assert_eq!(s.a, 8);

        s.redo();
        assert_eq!(s.a, 11);

        s.redo();
        assert_eq!(s.a, 11);

        s.undo();
        s.undo();
        assert_eq!(s.a, 3);

        s.undo();
        assert_eq!(s.a, 3);

        s.redo();
        assert_eq!(s.a, 8);
        s.a -= 10;
        assert_eq!(s.a, -2);
        s.undo();
        assert_eq!(s.a, 3); // An undo while there's an un-commited active edit will discard it
        s.redo();
        assert_eq!(s.a, 8);
        s.a -= 10;
        s.commit();
        assert_eq!(s.a, -2);
        s.redo();
        assert_eq!(s.a, -2); // Check that stuff got truncated on commit
    }
}
