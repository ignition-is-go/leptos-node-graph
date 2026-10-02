use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::Arc;

use leptos::prelude::*;

/// One authored change. A batch contains only the keys that changed.
#[derive(Clone, Debug)]
pub enum KeyedChange<K, V> {
    Upsert(K, V),
    Remove(K),
}

type Listener<K, V> = Arc<dyn Fn(&K, Option<&V>, Option<&V>) + Send + Sync>;

struct State<K, V> {
    values: HashMap<K, V>,
    points: HashMap<K, ArcRwSignal<Option<V>>>,
    listeners: HashMap<usize, Listener<K, V>>,
    next_listener: usize,
    pending: VecDeque<Vec<KeyedChange<K, V>>>,
    draining: bool,
}

/// Membership changes notify `keys`; value changes notify only that key.
/// Missing point reads retain a signal so later insertion and removal are observable.
pub struct KeyedMap<K: 'static, V: 'static> {
    state: StoredValue<State<K, V>>,
    membership: RwSignal<Vec<K>>,
}

impl<K, V> Copy for KeyedMap<K, V> {}
impl<K, V> Clone for KeyedMap<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, V> Default for KeyedMap<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> KeyedMap<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            state: StoredValue::new(State {
                values: HashMap::new(),
                points: HashMap::new(),
                listeners: HashMap::new(),
                next_listener: 0,
                pending: VecDeque::new(),
                draining: false,
            }),
            membership: RwSignal::new(Vec::new()),
        }
    }

    pub fn point(&self, key: &K) -> ArcRwSignal<Option<V>> {
        let mut point = None;
        self.state.update_value(|state| {
            let value = state.values.get(key).cloned();
            point = Some(
                state
                    .points
                    .entry(key.clone())
                    .or_insert_with(|| ArcRwSignal::new(value))
                    .clone(),
            );
        });
        point.unwrap_or_else(|| ArcRwSignal::new(None))
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.point(key).get()
    }
    pub fn get_untracked(&self, key: &K) -> Option<V> {
        self.state
            .with_value(|state| state.values.get(key).cloned())
    }
    pub fn keys(&self) -> Vec<K> {
        self.membership.get()
    }
    pub fn keys_untracked(&self) -> Vec<K> {
        self.membership.get_untracked()
    }
    pub fn len(&self) -> usize {
        self.state.with_value(|state| state.values.len())
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Imperative inspection for user gestures and initialization; never subscribes.
    pub fn with_untracked<R>(&self, read: impl FnOnce(&HashMap<K, V>) -> R) -> R {
        self.state.with_value(|state| read(&state.values))
    }

    pub fn insert(&self, key: K, value: V) {
        self.apply([KeyedChange::Upsert(key, value)]);
    }
    pub fn remove(&self, key: &K) {
        self.apply([KeyedChange::Remove(key.clone())]);
    }
    pub fn update(&self, key: &K, update: impl FnOnce(&mut V)) {
        if let Some(mut value) = self.get_untracked(key) {
            update(&mut value);
            self.insert(key.clone(), value);
        }
    }

    pub fn apply(&self, changes: impl IntoIterator<Item = KeyedChange<K, V>>) {
        let mut start = false;
        self.state.update_value(|state| {
            state.pending.push_back(changes.into_iter().collect());
            if !state.draining {
                state.draining = true;
                start = true;
            }
        });
        if !start {
            return;
        }
        loop {
            let mut next = None;
            self.state.update_value(|state| {
                next = state.pending.pop_front();
                if next.is_none() {
                    state.draining = false;
                }
            });
            let Some(changes) = next else {
                break;
            };
            let mut notifications = Vec::new();
            let mut membership_changed = false;
            self.state.update_value(|state| {
                for change in changes {
                    let (key, value) = match change {
                        KeyedChange::Upsert(key, value) => (key, Some(value)),
                        KeyedChange::Remove(key) => (key, None),
                    };
                    let old = state.values.get(&key).cloned();
                    if old == value {
                        continue;
                    }
                    membership_changed |= old.is_some() != value.is_some();
                    match &value {
                        Some(value) => {
                            state.values.insert(key.clone(), value.clone());
                        }
                        None => {
                            state.values.remove(&key);
                        }
                    }
                    let point = state.points.get(&key).cloned();
                    let listeners: Vec<_> = state.listeners.values().cloned().collect();
                    notifications.push((key, old, value, point, listeners));
                }
            });
            batch(move || {
                if membership_changed {
                    self.membership.set(
                        self.state
                            .with_value(|state| state.values.keys().cloned().collect()),
                    );
                }
                // Release the store before notifications: listeners may author another edit.
                for (key, old, value, point, listeners) in notifications {
                    if let Some(point) = point {
                        point.set(value.clone());
                    }
                    for listener in listeners {
                        listener(&key, old.as_ref(), value.as_ref());
                    }
                }
            });
        }
    }

    /// Receive changed rows directly. The listener is removed when this owner is disposed.
    pub fn subscribe(&self, listener: impl Fn(&K, Option<&V>, Option<&V>) + Send + Sync + 'static) {
        let mut id = 0;
        self.state.update_value(|state| {
            id = state.next_listener;
            state.next_listener += 1;
            state.listeners.insert(id, Arc::new(listener));
        });
        let state = self.state;
        on_cleanup(move || {
            state.update_value(|state| {
                state.listeners.remove(&id);
            })
        });
    }
}

/// Selection publishes a boolean independently for each selected identity.
pub struct KeyedSet<K: 'static>(pub KeyedMap<K, ()>);
impl<K> Copy for KeyedSet<K> {}
impl<K> Clone for KeyedSet<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Default for KeyedSet<K>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
{
    fn default() -> Self {
        Self(KeyedMap::new())
    }
}
impl<K> KeyedSet<K>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
{
    pub fn contains(&self, key: &K) -> bool {
        self.0.get(key).is_some()
    }
    pub fn contains_untracked(&self, key: &K) -> bool {
        self.0.get_untracked(key).is_some()
    }
    pub fn set(&self, keys: HashSet<K>) {
        let old: HashSet<K> = self.0.keys_untracked().into_iter().collect();
        self.0.apply(
            old.difference(&keys)
                .cloned()
                .map(KeyedChange::Remove)
                .chain(
                    keys.difference(&old)
                        .cloned()
                        .map(|key| KeyedChange::Upsert(key, ())),
                ),
        );
    }
    pub fn insert(&self, key: K) {
        self.0.insert(key, ());
    }
    pub fn remove(&self, key: &K) {
        self.0.remove(key);
    }
    pub fn update(&self, update: impl FnOnce(&mut HashSet<K>)) {
        let mut keys = self.keys_untracked().into_iter().collect();
        update(&mut keys);
        self.set(keys);
    }
    pub fn clear(&self) {
        self.0
            .apply(self.0.keys_untracked().into_iter().map(KeyedChange::Remove));
    }
    pub fn keys(&self) -> Vec<K> {
        self.0.keys()
    }
    pub fn keys_untracked(&self) -> Vec<K> {
        self.0.keys_untracked()
    }
    pub fn with_untracked<R>(&self, read: impl FnOnce(&HashSet<K>) -> R) -> R {
        read(&self.keys_untracked().into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn point_and_membership_reads_are_independent_and_missing_keys_revive() {
        Owner::new().with(|| {
            let map: KeyedMap<usize, usize> = KeyedMap::new();
            let reads = Arc::new(AtomicUsize::new(0));
            let count = reads.clone();
            let point = Memo::new(move |_| {
                count.fetch_add(1, Ordering::Relaxed);
                map.get(&1)
            });
            let memberships = Arc::new(AtomicUsize::new(0));
            let count = memberships.clone();
            let keys = Memo::new(move |_| {
                count.fetch_add(1, Ordering::Relaxed);
                map.keys().len()
            });
            assert_eq!(point.get(), None);
            assert_eq!(keys.get(), 0);
            map.insert(2, 2);
            assert_eq!(point.get(), None);
            assert_eq!(keys.get(), 1);
            assert_eq!(reads.load(Ordering::Relaxed), 1);
            map.insert(2, 3);
            assert_eq!(keys.get(), 1);
            assert_eq!(memberships.load(Ordering::Relaxed), 2);
            map.insert(1, 4);
            assert_eq!(point.get(), Some(4));
            map.remove(&1);
            assert_eq!(point.get(), None);
            map.insert(1, 5);
            assert_eq!(point.get(), Some(5));
            map.insert(1, 5);
            assert_eq!(point.get(), Some(5));
            assert_eq!(reads.load(Ordering::Relaxed), 4);
        });
    }

    #[test]
    fn reentrant_listener_edits_drain_after_the_current_batch() {
        Owner::new().with(|| {
            let map: KeyedMap<usize, usize> = KeyedMap::new();
            let observed: StoredValue<Vec<(usize, Option<usize>)>> = StoredValue::new(Vec::new());
            map.subscribe(move |key, _, value| {
                observed.update_value(|events| events.push((*key, value.copied())));
                if *key == 1 && value == Some(&1) {
                    map.insert(1, 2);
                    map.remove(&3);
                }
            });
            map.apply([KeyedChange::Upsert(1, 1), KeyedChange::Upsert(3, 3)]);
            assert_eq!(map.get(&1), Some(2));
            assert_eq!(map.get(&3), None);
            assert_eq!(
                observed.get_value(),
                vec![(1, Some(1)), (3, Some(3)), (1, Some(2)), (3, None)]
            );
        });
    }
}
