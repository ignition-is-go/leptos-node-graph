use std::collections::{HashMap, HashSet};

use leptos::prelude::*;

use crate::types::*;

/// Entry for a registered node.
#[derive(Clone)]
pub struct NodeEntry<N: NodeId> {
    pub id: N,
    pub position: Position,
    pub size: Size,
    /// The consumer's position signal — updated during drag for live visual feedback.
    pub position_signal: Option<RwSignal<Position>>,
}

/// Entry for a registered port.
#[derive(Clone, Debug)]
pub struct PortEntry<N: NodeId, P: PortId, T: PortType> {
    pub id: P,
    pub node_id: N,
    pub direction: PortDirection,
    pub port_type: T,
    pub position: Position,
}

/// A connection between two ports.
#[derive(Clone, Debug)]
pub struct ConnectionEntry<P: PortId, C: ConnectionId> {
    pub id: C,
    pub source: P,
    pub target: P,
}

/// Box select state.
#[derive(Clone, Debug)]
pub struct BoxSelect {
    pub start: Position,
    pub current: Position,
}

impl BoxSelect {
    /// Compute the axis-aligned bounding rectangle from start and current positions.
    pub fn to_rect(&self) -> Rect {
        let min_x = self.start.x.min(self.current.x);
        let min_y = self.start.y.min(self.current.y);
        let max_x = self.start.x.max(self.current.x);
        let max_y = self.start.y.max(self.current.y);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Drag state for node dragging.
#[derive(Clone, Debug)]
pub struct DragState<N: NodeId> {
    pub node_id: N,
    pub offset: Position,
    pub start_positions: HashMap<N, Position>,
    /// Whether alt was held when the drag started.
    pub alt_key: bool,
}

/// The central reactive state store.
#[derive(Clone)]
pub struct EditorRegistry<N, P, C, T>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    pub nodes: RwSignal<HashMap<N, NodeEntry<N>>>,
    pub ports: RwSignal<HashMap<P, PortEntry<N, P, T>>>,
    pub connections: RwSignal<HashMap<C, ConnectionEntry<P, C>>>,
    pub selected_nodes: RwSignal<HashSet<N>>,
    pub selected_connections: RwSignal<HashSet<C>>,
    pub draft_connection: RwSignal<Option<DraftConnection<P, T>>>,
    pub viewport: RwSignal<ViewportTransform>,
    pub config: RwSignal<EditorConfig>,
    pub on_event: StoredValue<Callback<GraphEvent<N, P, C>>>,
    pub box_select: RwSignal<Option<BoxSelect>>,
    pub drag_state: RwSignal<Option<DragState<N>>>,
}

impl<N, P, C, T> EditorRegistry<N, P, C, T>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    /// Create a new registry with the given configuration and event callback.
    pub fn new(config: EditorConfig, on_event: Callback<GraphEvent<N, P, C>>) -> Self {
        Self {
            nodes: RwSignal::new(HashMap::new()),
            ports: RwSignal::new(HashMap::new()),
            connections: RwSignal::new(HashMap::new()),
            selected_nodes: RwSignal::new(HashSet::new()),
            selected_connections: RwSignal::new(HashSet::new()),
            draft_connection: RwSignal::new(None),
            viewport: RwSignal::new(ViewportTransform::default()),
            config: RwSignal::new(config),
            on_event: StoredValue::new(on_event),
            box_select: RwSignal::new(None),
            drag_state: RwSignal::new(None),
        }
    }

    /// Emit an event through the on_event callback.
    pub fn emit(&self, event: GraphEvent<N, P, C>) {
        self.on_event.with_value(|cb| cb.run(event));
    }

    /// Register a node at the given position.
    pub fn register_node(&self, id: N, position: Position, position_signal: Option<RwSignal<Position>>) {
        self.nodes.update(|nodes| {
            nodes.insert(
                id.clone(),
                NodeEntry {
                    id,
                    position,
                    size: Size::default(),
                    position_signal,
                },
            );
        });
    }

    /// Deregister a node, cascading removal of its ports and their connections.
    pub fn deregister_node(&self, id: &N) {
        // Collect ports belonging to this node.
        let port_ids: Vec<P> = self.ports.with_untracked(|ports| {
            ports
                .values()
                .filter(|entry| &entry.node_id == id)
                .map(|entry| entry.id.clone())
                .collect()
        });

        // Deregister each port (which also removes its connections).
        for port_id in port_ids {
            self.deregister_port(&port_id);
        }

        self.nodes.update(|nodes| {
            nodes.remove(id);
        });

        self.selected_nodes.update(|sel| {
            sel.remove(id);
        });
    }

    /// Register a port.
    pub fn register_port(
        &self,
        id: P,
        node_id: N,
        direction: PortDirection,
        port_type: T,
        position: Position,
    ) {
        self.ports.update(|ports| {
            ports.insert(
                id.clone(),
                PortEntry {
                    id,
                    node_id,
                    direction,
                    port_type,
                    position,
                },
            );
        });
    }

    /// Deregister a port and remove any connections that reference it,
    /// emitting `ConnectionRemoved` events for each.
    pub fn deregister_port(&self, id: &P) {
        // Find connections referencing this port.
        let to_remove: Vec<(C, C)> = self.connections.with_untracked(|conns| {
            conns
                .values()
                .filter(|c| &c.source == id || &c.target == id)
                .map(|c| (c.id.clone(), c.id.clone()))
                .collect()
        });

        for (conn_id, event_id) in to_remove {
            self.connections.update(|conns| {
                conns.remove(&conn_id);
            });
            self.selected_connections.update(|sel| {
                sel.remove(&conn_id);
            });
            self.emit(GraphEvent::ConnectionRemoved { id: event_id });
        }

        self.ports.update(|ports| {
            ports.remove(id);
        });
    }

    /// Update a node's position.
    pub fn set_node_position(&self, id: &N, position: Position) {
        self.nodes.update(|nodes| {
            if let Some(entry) = nodes.get_mut(id) {
                entry.position = position;
            }
        });
    }

    /// Update a node's position and its consumer signal. Used during drag for live feedback.
    pub fn set_node_position_with_signal(&self, id: &N, position: Position) {
        let signal = self.nodes.with_untracked(|nodes| {
            nodes.get(id).and_then(|e| e.position_signal)
        });
        self.nodes.update(|nodes| {
            if let Some(entry) = nodes.get_mut(id) {
                entry.position = position;
            }
        });
        if let Some(sig) = signal {
            sig.set(position);
        }
    }

    /// Update a node's size.
    pub fn set_node_size(&self, id: &N, size: Size) {
        self.nodes.update(|nodes| {
            if let Some(entry) = nodes.get_mut(id) {
                entry.size = size;
            }
        });
    }

    /// Update a port's absolute canvas-space position.
    pub fn set_port_position(&self, id: &P, position: Position) {
        self.ports.update(|ports| {
            if let Some(entry) = ports.get_mut(id) {
                entry.position = position;
            }
        });
    }

    /// Replace the entire connections map (used by the consumer to sync state).
    pub fn set_connections(&self, connections: HashMap<C, ConnectionEntry<P, C>>) {
        self.connections.set(connections);
    }

    /// Get a port's position (untracked read).
    pub fn port_position(&self, id: &P) -> Option<Position> {
        self.ports.with_untracked(|ports| ports.get(id).map(|e| e.position))
    }

    /// Get a clone of a port entry (untracked read).
    pub fn get_port(&self, id: &P) -> Option<PortEntry<N, P, T>> {
        self.ports.with_untracked(|ports| ports.get(id).cloned())
    }

    /// Check if a target port is compatible with the current draft connection.
    ///
    /// Requirements:
    /// - Target must be an Input port.
    /// - Target must not belong to the same node as the source.
    /// - The port types must be compatible via `PortType::compatible`.
    pub fn is_compatible_target(&self, target_id: &P) -> bool {
        let draft = self.draft_connection.with_untracked(|d| d.clone());
        let Some(draft) = draft else {
            return false;
        };

        self.ports.with_untracked(|ports| {
            let Some(target) = ports.get(target_id) else {
                return false;
            };

            if target.direction != PortDirection::Input {
                return false;
            }

            // Source port must exist and belong to a different node.
            let Some(source) = ports.get(&draft.source_port) else {
                return false;
            };

            if source.node_id == target.node_id {
                return false;
            }

            T::compatible(&draft.port_type, &target.port_type)
        })
    }

    /// Select a single node, clearing all other selections.
    pub fn select_node(&self, id: N) {
        self.selected_connections.update(|sel| sel.clear());
        self.selected_nodes.update(|sel| {
            sel.clear();
            sel.insert(id);
        });
    }

    /// Toggle a node's selection state (for shift+click).
    pub fn toggle_node_selection(&self, id: N) {
        self.selected_nodes.update(|sel| {
            if !sel.remove(&id) {
                sel.insert(id);
            }
        });
    }

    /// Select a single connection, clearing all other selections.
    pub fn select_connection(&self, id: C) {
        self.selected_nodes.update(|sel| sel.clear());
        self.selected_connections.update(|sel| {
            sel.clear();
            sel.insert(id);
        });
    }

    /// Clear all selections.
    pub fn clear_selection(&self) {
        self.selected_nodes.update(|sel| sel.clear());
        self.selected_connections.update(|sel| sel.clear());
    }

    /// Select all nodes.
    pub fn select_all(&self) {
        let all_ids: HashSet<N> = self.nodes.with_untracked(|nodes| nodes.keys().cloned().collect());
        self.selected_nodes.set(all_ids);
    }

    /// Emit deletion events for all selected items.
    pub fn delete_selected(&self) {
        let selected_node_ids: Vec<N> = self
            .selected_nodes
            .with_untracked(|sel| sel.iter().cloned().collect());

        let selected_conn_ids: Vec<C> = self
            .selected_connections
            .with_untracked(|sel| sel.iter().cloned().collect());

        // Emit connection removals first.
        for conn_id in selected_conn_ids {
            self.emit(GraphEvent::ConnectionRemoved { id: conn_id });
        }

        // Emit node deletions.
        if !selected_node_ids.is_empty() {
            self.emit(GraphEvent::NodesDeleted {
                ids: selected_node_ids,
            });
        }
    }

    /// Return the set of node IDs whose bounding rectangles intersect the given rect.
    pub fn nodes_in_rect(&self, rect: &Rect) -> HashSet<N> {
        self.nodes.with_untracked(|nodes| {
            nodes
                .values()
                .filter(|entry| {
                    let node_rect = Rect::new(
                        entry.position.x,
                        entry.position.y,
                        entry.size.width,
                        entry.size.height,
                    );
                    rect.intersects(&node_rect)
                })
                .map(|entry| entry.id.clone())
                .collect()
        })
    }
}
