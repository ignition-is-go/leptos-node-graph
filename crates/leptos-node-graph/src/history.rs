use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct HistoryCommand<T: Clone + 'static> {
    pub data: T,
}

#[derive(Clone)]
pub struct UndoHistory<T: Clone + Send + Sync + 'static> {
    undo_stack: RwSignal<Vec<HistoryCommand<T>>>,
    redo_stack: RwSignal<Vec<HistoryCommand<T>>>,
    max_size: usize,
}

impl<T: Clone + Send + Sync + 'static> UndoHistory<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: RwSignal::new(Vec::new()),
            redo_stack: RwSignal::new(Vec::new()),
            max_size,
        }
    }

    pub fn push(&self, data: T) {
        self.redo_stack.set(Vec::new());
        self.undo_stack.update(|stack| {
            if stack.len() >= self.max_size {
                stack.remove(0);
            }
            stack.push(HistoryCommand { data });
        });
    }

    pub fn undo(&self) -> Option<T> {
        let cmd = self.undo_stack.try_update(|stack| stack.pop()).flatten();
        if let Some(ref cmd) = cmd {
            self.redo_stack.update(|stack| {
                stack.push(HistoryCommand { data: cmd.data.clone() });
            });
        }
        cmd.map(|c| c.data)
    }

    pub fn redo(&self) -> Option<T> {
        let cmd = self.redo_stack.try_update(|stack| stack.pop()).flatten();
        if let Some(ref cmd) = cmd {
            self.undo_stack.update(|stack| {
                stack.push(HistoryCommand { data: cmd.data.clone() });
            });
        }
        cmd.map(|c| c.data)
    }

    pub fn can_undo(&self) -> bool {
        self.undo_stack.with_untracked(|s| !s.is_empty())
    }

    pub fn can_redo(&self) -> bool {
        self.redo_stack.with_untracked(|s| !s.is_empty())
    }

    pub fn clear(&self) {
        self.undo_stack.set(Vec::new());
        self.redo_stack.set(Vec::new());
    }
}
