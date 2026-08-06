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
    /// Index of this port among its node's ports of the same direction.
    pub slot_index: usize,
    /// Cached offset from node position. Set by the anchor's first DOM measurement.
    pub offset: Option<Position>,
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

/// Width-resize state for one node. Started by that node's right-edge handle,
/// driven by the document-level mousemove, cleared on mouseup.
#[derive(Clone)]
pub struct ResizeState<N: NodeId> {
    pub node_id: N,
    /// Canvas-space x where the gesture started.
    pub start_x: f64,
    /// The node's rendered width when the gesture started.
    pub start_width: f64,
    /// The node's width signal — written live so the node re-renders as it drags.
    pub width_signal: RwSignal<Option<f64>>,
    /// Clamp bounds, resolved from the theme when the gesture started.
    pub min_width: f64,
    pub max_width: Option<f64>,
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
    /// A DEBOUNCED mirror of `viewport`, updated only after pan/zoom settles.
    /// `NodeVisible` reads this (not the live `viewport`) so visibility — and thus
    /// the create/dispose of off-screen node content — never churns mid-pan (the
    /// live viewport still drives the smooth CSS transform every frame).
    pub visibility_viewport: RwSignal<ViewportTransform>,
    /// Measured pixel size of the editor container (screen space). Written by the
    /// editor; read by nodes to compute their viewport visibility (`NodeVisible`).
    pub container_size: RwSignal<Size>,
    pub config: RwSignal<EditorConfig>,
    pub on_event: StoredValue<Callback<GraphEvent<N, P, C>>>,
    pub box_select: RwSignal<Option<BoxSelect>>,
    /// Whether THIS editor instance is mid-pan. Set by the container's mousedown
    /// (so it's scoped to the pressed graph) and checked by the document-level
    /// mousemove — without it, a pan would move every open graph at once, since the
    /// move listener is global.
    pub is_panning: RwSignal<bool>,
    /// Client-space pointer position from the previous pan frame.
    ///
    /// Pan deltas are measured against this rather than taken from
    /// `MouseEvent.movementX/Y`: Chrome reports those in PHYSICAL device pixels
    /// while `clientX/Y` are CSS pixels, so on a HiDPI screen (or with page
    /// zoom) `movementX` overstates the cursor's travel by the device pixel
    /// ratio and the canvas slides faster than the mouse.
    pub pan_origin: RwSignal<Option<Position>>,
    /// Whether the node-creation menu is open on this editor.
    ///
    /// Mirrored from the menu's own `open_at` signal so the interaction handlers
    /// can see it: an open menu takes ownership of the in-flight draft, and a
    /// mouseup that would otherwise cancel the draft has to leave it alone.
    pub menu_open: RwSignal<bool>,
    pub drag_state: RwSignal<Option<DragState<N>>>,
    /// In-flight node width resize, if any.
    pub resize_state: RwSignal<Option<ResizeState<N>>>,
    /// Pending drag canvas position — written by mousemove, applied by RAF.
    pub pending_drag_pos: RwSignal<Option<Position>>,
    /// Whether a RAF callback is already scheduled for drag.
    pub drag_raf_pending: RwSignal<bool>,
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
            visibility_viewport: RwSignal::new(ViewportTransform::default()),
            container_size: RwSignal::new(Size::default()),
            config: RwSignal::new(config),
            on_event: StoredValue::new(on_event),
            box_select: RwSignal::new(None),
            is_panning: RwSignal::new(false),
            pan_origin: RwSignal::new(None),
            menu_open: RwSignal::new(false),
            drag_state: RwSignal::new(None),
            resize_state: RwSignal::new(None),
            pending_drag_pos: RwSignal::new(None),
            drag_raf_pending: RwSignal::new(false),
        }
    }

    /// Emit an event through the on_event callback.
    pub fn emit(&self, event: GraphEvent<N, P, C>) {
        self.on_event.with_value(|cb| cb.run(event));
    }

    /// Register a node at the given position.
    pub fn register_node(
        &self,
        id: N,
        position: Position,
        position_signal: Option<RwSignal<Position>>,
    ) {
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

    /// Register a port. Invalidates cached offsets for all sibling ports on the same node
    /// since the new port may shift their visual positions.
    pub fn register_port(
        &self,
        id: P,
        node_id: N,
        direction: PortDirection,
        port_type: T,
        position: Position,
    ) {
        self.ports.update(|ports| {
            // New port gets the next index for this node+direction
            let next_idx = ports
                .values()
                .filter(|p| p.node_id == node_id && p.direction == direction)
                .count();

            // Invalidate offsets for existing sibling ports (positions may shift)
            for entry in ports.values_mut() {
                if entry.node_id == node_id {
                    entry.offset = None;
                }
            }

            ports.insert(
                id.clone(),
                PortEntry {
                    id,
                    node_id,
                    direction,
                    port_type,
                    position,
                    slot_index: next_idx,
                    offset: None,
                },
            );
        });
    }

    /// Deregister a port. Connections referencing this port are kept in the
    /// consumer's data but not rendered (the connection renderer skips connections
    /// with missing ports). This allows connections to restore if the port reappears.
    pub fn deregister_port(&self, id: &P) {
        // Get the node_id and direction before removing
        let port_info = self
            .ports
            .with_untracked(|ports| ports.get(id).map(|p| (p.node_id.clone(), p.direction)));

        self.ports.update(|ports| {
            ports.remove(id);

            // Reindex siblings preserving their original order
            if let Some((node_id, direction)) = &port_info {
                let mut siblings: Vec<(usize, P)> = ports
                    .values()
                    .filter(|p| &p.node_id == node_id && p.direction == *direction)
                    .map(|p| (p.slot_index, p.id.clone()))
                    .collect();
                siblings.sort_by_key(|(idx, _)| *idx);

                for (new_idx, (_, port_id)) in siblings.into_iter().enumerate() {
                    if let Some(entry) = ports.get_mut(&port_id) {
                        entry.slot_index = new_idx;
                        entry.offset = None;
                    }
                }
            }
        });
    }

    /// Update a node's position.
    pub fn set_node_position(&self, id: &N, position: Position) {
        self.nodes.maybe_update(|nodes| {
            let Some(entry) = nodes.get_mut(id) else {
                return false;
            };
            if entry.position == position {
                return false;
            }
            entry.position = position;
            true
        });
    }

    /// Update a node's position and its consumer signal. Used during drag for live feedback.
    pub fn set_node_position_with_signal(&self, id: &N, position: Position) {
        let mut signal = None;
        self.nodes.maybe_update(|nodes| {
            let Some(entry) = nodes.get_mut(id) else {
                return false;
            };
            signal = entry.position_signal;
            if entry.position == position {
                return false;
            }
            entry.position = position;
            true
        });
        if let Some(signal) = signal
            && signal.get_untracked() != position
        {
            signal.set(position);
        }
    }

    /// Batch-update positions for multiple nodes during drag.
    /// Updates node entries, port positions (via cached offsets), and position signals.
    /// Each map notifies at most once, and only when at least one value changed.
    pub fn batch_set_positions(&self, updates: &[(N, Position)]) {
        // 1. Update node entries and collect only consumer signals that changed.
        let mut node_signals = Vec::new();
        let mut node_moves = HashMap::new();
        self.nodes.maybe_update(|nodes| {
            let mut changed = false;
            for (id, position) in updates {
                let Some(entry) = nodes.get_mut(id) else {
                    continue;
                };
                if entry.position == *position {
                    if let Some(signal) = entry.position_signal
                        && signal.get_untracked() != *position
                    {
                        node_signals.push((signal, *position));
                    }
                    continue;
                }
                let delta =
                    Position::new(position.x - entry.position.x, position.y - entry.position.y);
                entry.position = *position;
                node_moves.insert(id.clone(), (*position, delta));
                if let Some(signal) = entry.position_signal {
                    node_signals.push((signal, *position));
                }
                changed = true;
            }
            changed
        });

        // 2. Batch-update port positions. Measured ports use their exact cached
        // offset; dynamic siblings whose offset was invalidated still follow
        // the node by translating their last known absolute position.
        self.ports.maybe_update(|ports| {
            let mut changed = false;
            for entry in ports.values_mut() {
                let Some((new_node_position, delta)) = node_moves.get(&entry.node_id) else {
                    continue;
                };
                let position = match entry.offset {
                    Some(offset) => Position::new(
                        new_node_position.x + offset.x,
                        new_node_position.y + offset.y,
                    ),
                    None => Position::new(entry.position.x + delta.x, entry.position.y + delta.y),
                };
                if entry.position != position {
                    entry.position = position;
                    changed = true;
                }
            }
            changed
        });

        // 3. Set position signals (drives CSS node positioning via style=).
        for (signal, position) in node_signals {
            if signal.get_untracked() != position {
                signal.set(position);
            }
        }
    }

    /// Update a node's size.
    pub fn set_node_size(&self, id: &N, size: Size) {
        self.nodes.maybe_update(|nodes| {
            let Some(entry) = nodes.get_mut(id) else {
                return false;
            };
            if entry.size == size {
                return false;
            }
            entry.size = size;
            true
        });
    }

    /// Update a port's absolute canvas-space position.
    pub fn set_port_position(&self, id: &P, position: Position) {
        self.ports.maybe_update(|ports| {
            let Some(entry) = ports.get_mut(id) else {
                return false;
            };
            if entry.position == position {
                return false;
            }
            entry.position = position;
            true
        });
    }

    /// Set a port's cached offset from its node position.
    pub fn set_port_offset(&self, id: &P, offset: Position) {
        let mut guard = self.ports.write_untracked();
        if let Some(entry) = guard.get_mut(id) {
            entry.offset = Some(offset);
        }
    }

    /// Replace the entire connections map (used by the consumer to sync state).
    pub fn set_connections(&self, connections: HashMap<C, ConnectionEntry<P, C>>) {
        self.connections.set(connections);
    }

    /// Get a port's position (untracked read).
    pub fn port_position(&self, id: &P) -> Option<Position> {
        self.ports
            .with_untracked(|ports| ports.get(id).map(|e| e.position))
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
        let all_ids: HashSet<N> = self
            .nodes
            .with_untracked(|nodes| nodes.keys().cloned().collect());
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
    /// The compatible port nearest `cursor` within `max_dist` (canvas units),
    /// for the draft currently in flight.
    ///
    /// Applies exactly the rules a real connection would — opposite direction,
    /// different node, type-compatible — so the wire can never snap to a port
    /// that would refuse the connection on release.
    pub fn snap_target_for_draft(&self, cursor: Position, max_dist: f64) -> Option<(P, Position)> {
        let draft = self.draft_connection.with_untracked(|d| d.clone())?;
        let source = self.get_port(&draft.source_port)?;

        let mut best: Option<(P, Position, f64)> = None;
        self.ports.with_untracked(|ports| {
            for entry in ports.values() {
                if entry.direction == draft.origin_direction || entry.node_id == source.node_id {
                    continue;
                }
                let (output_type, input_type) = if draft.origin_direction == PortDirection::Output {
                    (&source.port_type, &entry.port_type)
                } else {
                    (&entry.port_type, &source.port_type)
                };
                if !T::compatible(output_type, input_type) {
                    continue;
                }
                let dist = crate::utils::distance(cursor, entry.position);
                if dist <= max_dist && best.as_ref().is_none_or(|(_, _, b)| dist < *b) {
                    best = Some((entry.id.clone(), entry.position, dist));
                }
            }
        });
        best.map(|(id, pos, _)| (id, pos))
    }

    /// Bounding box of every registered node, in canvas space. `None` when the
    /// graph is empty.
    pub fn nodes_bounds(&self) -> Option<Rect> {
        self.nodes.with_untracked(|nodes| {
            let mut entries = nodes.values();
            let first = entries.next()?;
            let (mut min_x, mut min_y) = (first.position.x, first.position.y);
            let mut max_x = first.position.x + first.size.width;
            let mut max_y = first.position.y + first.size.height;
            for entry in entries {
                min_x = min_x.min(entry.position.x);
                min_y = min_y.min(entry.position.y);
                max_x = max_x.max(entry.position.x + entry.size.width);
                max_y = max_y.max(entry.position.y + entry.size.height);
            }
            Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
        })
    }

    /// Pan and zoom so `rect` (canvas space) is centered and fits the container,
    /// inset by `padding` screen pixels. Zoom is clamped to the config's range,
    /// and additionally to `max_zoom_override` when given — framing uses that to
    /// avoid magnifying a small graph.
    ///
    /// No-op until the container has been measured.
    pub fn fit_rect(&self, rect: &Rect, padding: f64, max_zoom_override: Option<f64>) {
        let container = self.container_size.get_untracked();
        if container.width <= 0.0 || container.height <= 0.0 {
            return;
        }

        // A single zero-size node would otherwise divide by zero.
        let width = rect.size.width.max(1.0);
        let height = rect.size.height.max(1.0);
        let avail_w = (container.width - 2.0 * padding).max(1.0);
        let avail_h = (container.height - 2.0 * padding).max(1.0);

        let (min_zoom, max_zoom) = self.config.with_untracked(|c| (c.min_zoom, c.max_zoom));
        let ceiling = max_zoom_override.map_or(max_zoom, |m| m.min(max_zoom));
        let zoom = (avail_w / width)
            .min(avail_h / height)
            .clamp(min_zoom, ceiling.max(min_zoom));

        let center_x = rect.position.x + width / 2.0;
        let center_y = rect.position.y + height / 2.0;
        self.viewport.update(|vp| {
            vp.zoom = zoom;
            vp.pan_x = container.width / 2.0 - center_x * zoom;
            vp.pan_y = container.height / 2.0 - center_y * zoom;
        });
    }

    /// Frame the whole graph — the `F` hotkey. No-op on an empty graph.
    pub fn fit_view(&self) {
        let Some(bounds) = self.nodes_bounds() else {
            return;
        };
        let (padding, fit_max_zoom) = self
            .config
            .with_untracked(|c| (c.fit_padding, c.fit_max_zoom));
        self.fit_rect(&bounds, padding, Some(fit_max_zoom));
    }

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum TestPortType {
        Any,
    }

    impl PortType for TestPortType {
        fn compatible(_: &Self, _: &Self) -> bool {
            true
        }

        fn type_id(&self) -> String {
            "any".into()
        }

        fn from_type_id(_: &str) -> Self {
            Self::Any
        }
    }

    #[test]
    fn geometry_writes_gate_noops_and_translate_offsetless_ports() {
        Owner::new().with(|| {
            let registry = EditorRegistry::<String, String, String, TestPortType>::new(
                EditorConfig::default(),
                Callback::new(|_: GraphEvent<String, String, String>| {}),
            );
            let position = Position::new(10.0, 20.0);
            let position_signal = RwSignal::new(position);
            let node_id = "node".to_string();
            let port_id = "port".to_string();
            let dynamic_port_id = "dynamic-port".to_string();
            registry.register_node(node_id.clone(), position, Some(position_signal));
            registry.register_port(
                port_id.clone(),
                node_id.clone(),
                PortDirection::Output,
                TestPortType::Any,
                Position::new(30.0, 25.0),
            );
            registry.register_port(
                dynamic_port_id.clone(),
                node_id.clone(),
                PortDirection::Output,
                TestPortType::Any,
                Position::new(50.0, 60.0),
            );
            registry.set_port_offset(&port_id, Position::new(20.0, 5.0));

            let node_reads = Arc::new(AtomicUsize::new(0));
            let node_reads_memo = Arc::clone(&node_reads);
            let nodes = registry.nodes;
            let node_geometry = Memo::new(move |_| {
                node_reads_memo.fetch_add(1, Ordering::Relaxed);
                nodes.with(|nodes| nodes.get("node").map(|node| (node.position, node.size)))
            });

            let port_reads = Arc::new(AtomicUsize::new(0));
            let port_reads_memo = Arc::clone(&port_reads);
            let ports = registry.ports;
            let port_geometry = Memo::new(move |_| {
                port_reads_memo.fetch_add(1, Ordering::Relaxed);
                ports.with(|ports| {
                    Some((
                        ports.get("port")?.position,
                        ports.get("dynamic-port")?.position,
                    ))
                })
            });

            assert!(node_geometry.get().is_some());
            assert!(port_geometry.get().is_some());
            registry.set_node_position(&node_id, position);
            registry.set_node_position_with_signal(&node_id, position);
            registry.set_node_size(&node_id, Size::default());
            registry.set_port_position(&port_id, Position::new(30.0, 25.0));
            registry.batch_set_positions(&[("node".into(), position)]);
            assert!(node_geometry.get().is_some());
            assert!(port_geometry.get().is_some());

            assert_eq!(node_reads.load(Ordering::Relaxed), 1);
            assert_eq!(port_reads.load(Ordering::Relaxed), 1);

            let moved = Position::new(20.0, 30.0);
            registry.batch_set_positions(&[(node_id.clone(), moved)]);
            assert_eq!(node_geometry.get(), Some((moved, Size::default())));
            assert_eq!(
                port_geometry.get(),
                Some((Position::new(40.0, 35.0), Position::new(60.0, 70.0)))
            );
            assert_eq!(position_signal.get_untracked(), moved);
            assert_eq!(node_reads.load(Ordering::Relaxed), 2);
            assert_eq!(port_reads.load(Ordering::Relaxed), 2);
        });
    }
}
