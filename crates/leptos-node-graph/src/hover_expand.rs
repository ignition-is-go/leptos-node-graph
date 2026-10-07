//! Delayed disclosure while a draft connection hovers over a compound row.
use leptos::prelude::*;
use std::time::Duration;

#[derive(Clone, Copy, Default)]
struct HoverIntent {
    sequence: u64,
    pending: Option<u64>,
}

impl HoverIntent {
    fn arm(&mut self) -> Option<u64> {
        if self.pending.is_some() {
            return None;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.pending = Some(self.sequence);
        self.pending
    }

    fn cancel(&mut self) {
        self.pending = None;
    }

    fn complete(&mut self, ticket: u64, eligible: bool) -> bool {
        if self.pending != Some(ticket) {
            return false;
        }
        self.cancel();
        eligible
    }
}

fn cancel_timer(timer: RwSignal<Option<TimeoutHandle>>) {
    if let Some(Some(handle)) = timer.try_get_untracked() {
        handle.clear();
        timer.try_set(None);
    }
}

/// Returns the row's hover signal. Set it on pointer enter/leave. Hovering only
/// opens a collapsed row during a draft, and never automatically closes it.
pub fn use_draft_hover_expansion(
    expanded: RwSignal<bool>,
    draft_active: Signal<bool>,
    delay: Duration,
) -> RwSignal<bool> {
    let hovered = RwSignal::new(false);
    let intent = RwSignal::new(HoverIntent::default());
    let timer = RwSignal::new(None);
    Effect::new(move |_| {
        let eligible = hovered.get() && !expanded.get() && draft_active.get();
        let mut state = intent.get_untracked();
        if !eligible {
            state.cancel();
            intent.set(state);
            cancel_timer(timer);
            return;
        }
        let Some(ticket) = state.arm() else {
            return;
        };
        intent.set(state);
        let scheduled = set_timeout_with_handle(
            move || {
                let Some(mut state) = intent.try_get_untracked() else {
                    return;
                };
                if state.pending != Some(ticket) {
                    return;
                }
                let eligible = hovered.try_get_untracked() == Some(true)
                    && expanded.try_get_untracked() == Some(false)
                    && draft_active.try_get_untracked() == Some(true);
                let open = state.complete(ticket, eligible);
                intent.try_set(state);
                timer.try_set(None);
                if open {
                    expanded.try_set(true);
                }
            },
            delay,
        );
        match scheduled {
            Ok(handle) => timer.set(Some(handle)),
            Err(_) => {
                state.cancel();
                intent.set(state);
            }
        }
    });
    on_cleanup(move || cancel_timer(timer));
    hovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_moves_do_not_restart_the_pending_delay() {
        let mut state = HoverIntent::default();
        let first = state.arm();
        assert!(first.is_some());
        assert_eq!(state.arm(), None);
        assert!(state.complete(first.unwrap_or_default(), true));
        assert!(!state.complete(first.unwrap_or_default(), true));
    }

    #[test]
    fn canceled_hover_cannot_expand_a_later_hover_or_drag() {
        let mut state = HoverIntent::default();
        let old = state.arm().unwrap_or_default();
        state.cancel();
        let new = state.arm().unwrap_or_default();
        assert_ne!(old, new);
        assert!(!state.complete(old, true));
        assert!(state.complete(new, true));
    }

    #[test]
    fn leaving_ending_drag_or_manual_expansion_blocks_even_a_queued_callback() {
        let mut state = HoverIntent::default();
        let ticket = state.arm().unwrap_or_default();
        assert!(!state.complete(ticket, false));
        assert!(!state.complete(ticket, true));
    }
}
