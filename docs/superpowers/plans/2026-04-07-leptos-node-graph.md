# leptos-node-graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a headless, trait-driven Leptos node graph editor library with a Trunk demo app.

**Architecture:** Context-provider pattern — `<NodeEditor>` provides reactive registry via `provide_context`, child `<Node>` and `<Anchor>` components register/deregister on mount/cleanup. HTML nodes + SVG connections + CSS transform pan/zoom. Consumer owns graph data, library owns UI state, events bridge the gap.

**Tech Stack:** Leptos 0.7+, wasm-bindgen, web-sys, Trunk (demo)

---

## File Map

### Library (`crates/leptos-node-graph/src/`)

| File | Responsibility |
|---|---|
| `types.rs` | Core traits (`PortType`, `NodeId`, `PortId`, `ConnectionId`), `GraphEvent` enum, `EditorConfig`, `Position`, `Size`, `Rect` geometry types |
| `registry.rs` | `EditorRegistry<T>` — reactive store of node entries, port entries, connections, selection set, draft connection, viewport transform |
| `editor.rs` | `<NodeEditor<T>>` component — root container, provides context, renders canvas + SVG overlay + children, handles global keyboard events |
| `node.rs` | `<Node>` component — registers in registry, provides `NodeContext`, handles drag, click-to-select, ResizeObserver |
| `anchor.rs` | `<InputAnchor<T>>` / `<OutputAnchor<T>>` — registers port, handles connection start/complete, compatibility highlighting |
| `connection.rs` | `<ConnectionRenderer>` — reads registry connections, renders SVG bezier paths, renders draft connection |
| `selection.rs` | `<SelectionBox>` — rubber-band box select overlay, selection state helpers |
| `interaction.rs` | Pan/zoom handler, drag handler, keyboard shortcut dispatcher |
| `history.rs` | `UndoHistory` — command stack, push/pop/undo/redo |
| `layout.rs` | `LayoutEngine` trait, `LayoutMode` enum, `LayoutGraph` data for layout computation |
| `utils.rs` | Bezier math, point-in-rect, distance-to-curve, snap-to-grid |
| `lib.rs` | Public re-exports |

### Demo (`examples/demo/`)

| File | Responsibility |
|---|---|
| `index.html` | Trunk entry point |
| `src/main.rs` | Demo app — sample node types, graph state, event handler, CSS styling |

---

## Task 1: Workspace and Crate Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/leptos-node-graph/Cargo.toml`
- Create: `crates/leptos-node-graph/src/lib.rs`
- Create: `examples/demo/Cargo.toml`
- Create: `examples/demo/index.html`
- Create: `examples/demo/src/main.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["crates/leptos-node-graph", "examples/demo"]
resolver = "2"
```

- [ ] **Step 2: Create library crate Cargo.toml**

```toml
[package]
name = "leptos-node-graph"
version = "0.1.0"
edition = "2024"

[dependencies]
leptos = { version = "0.7", features = ["csr"] }
web-sys = { version = "0.3", features = [
    "HtmlElement",
    "DomRect",
    "MouseEvent",
    "KeyboardEvent",
    "WheelEvent",
    "PointerEvent",
    "ResizeObserver",
    "ResizeObserverEntry",
    "ResizeObserverSize",
    "SvgsvgElement",
    "SvgPathElement",
    "Element",
    "CssStyleDeclaration",
] }
wasm-bindgen = "0.2"
```

- [ ] **Step 3: Create library src/lib.rs with module stubs**

```rust
pub mod types;
pub mod registry;
pub mod editor;
pub mod node;
pub mod anchor;
pub mod connection;
pub mod selection;
pub mod interaction;
pub mod history;
pub mod layout;
pub mod utils;

pub use types::*;
pub use editor::NodeEditor;
pub use node::Node;
pub use anchor::{InputAnchor, OutputAnchor};
pub use layout::{LayoutEngine, LayoutMode};
```

- [ ] **Step 4: Create empty module files**

Create each of these files with a placeholder comment so the crate compiles:

`crates/leptos-node-graph/src/types.rs`:
```rust
use std::collections::HashSet;
use std::hash::Hash;
```

`crates/leptos-node-graph/src/registry.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/editor.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/node.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/anchor.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/connection.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/selection.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/interaction.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/history.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/layout.rs`:
```rust
use crate::types::*;
```

`crates/leptos-node-graph/src/utils.rs`:
```rust
// Geometry utilities
```

- [ ] **Step 5: Create demo Cargo.toml**

```toml
[package]
name = "leptos-node-graph-demo"
version = "0.1.0"
edition = "2024"

[dependencies]
leptos = { version = "0.7", features = ["csr"] }
leptos-node-graph = { path = "../../crates/leptos-node-graph" }
web-sys = { version = "0.3", features = ["console"] }
wasm-bindgen = "0.2"
```

- [ ] **Step 6: Create demo index.html**

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>leptos-node-graph demo</title>
    <link data-trunk rel="rust" data-wasm-opt="z"/>
</head>
<body></body>
</html>
```

- [ ] **Step 7: Create demo src/main.rs stub**

```rust
use leptos::prelude::*;

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <div>"leptos-node-graph demo"</div>
    }
}
```

- [ ] **Step 8: Verify the workspace compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles with no errors (warnings for unused imports are fine)

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat: scaffold workspace with library crate and demo app"
```

---

## Task 2: Core Types and Traits

**Files:**
- Create: `crates/leptos-node-graph/src/types.rs`

- [ ] **Step 1: Write core traits and types**

Replace `crates/leptos-node-graph/src/types.rs` with:

```rust
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

/// Consumer implements this to define port type compatibility.
pub trait PortType: Clone + PartialEq + Debug + Send + Sync + 'static {
    /// Returns true if a connection from `source` output to `target` input is valid.
    fn compatible(source: &Self, target: &Self) -> bool;
}

/// Trait for node identifiers.
pub trait NodeId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}

/// Trait for port identifiers.
pub trait PortId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}

/// Trait for connection identifiers.
pub trait ConnectionId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}

/// Blanket implementations for common types.
impl NodeId for String {}
impl PortId for String {}
impl ConnectionId for String {}
impl NodeId for u64 {}
impl PortId for u64 {}
impl ConnectionId for u64 {}
impl NodeId for usize {}
impl PortId for usize {}
impl ConnectionId for usize {}

/// A 2D position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A 2D size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub position: Position,
    pub size: Size,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            position: Position::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn contains(&self, point: Position) -> bool {
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.width
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.position.x < other.position.x + other.size.width
            && self.position.x + self.size.width > other.position.x
            && self.position.y < other.position.y + other.size.height
            && self.position.y + self.size.height > other.position.y
    }
}

/// Direction of a port on a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

/// Events emitted by the editor to the consumer.
#[derive(Clone, Debug)]
pub enum GraphEvent<N, P, C>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
{
    NodeMoved { id: N, position: Position },
    NodeResized { id: N, size: Size },
    ConnectionRequested { source: P, target: P },
    ConnectionRemoved { id: C },
    SelectionChanged { nodes: HashSet<N>, connections: HashSet<C> },
    NodesDeleted { ids: Vec<N> },
    NodesCopied { ids: Vec<N> },
    NodesPasted { offset: Position },
    Undo,
    Redo,
    GroupCreated { node_ids: Vec<N> },
}

/// Layout mode for the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutMode {
    #[default]
    Classic,
    Structured,
}

/// Editor configuration.
#[derive(Clone, Debug)]
pub struct EditorConfig {
    /// Minimum zoom level (default: 0.1)
    pub min_zoom: f64,
    /// Maximum zoom level (default: 5.0)
    pub max_zoom: f64,
    /// Snap-to-grid size in pixels. None means no snapping.
    pub grid_size: Option<f64>,
    /// Layout mode.
    pub layout_mode: LayoutMode,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            min_zoom: 0.1,
            max_zoom: 5.0,
            grid_size: None,
            layout_mode: LayoutMode::Classic,
        }
    }
}

/// Viewport transform state.
#[derive(Clone, Copy, Debug)]
pub struct ViewportTransform {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for ViewportTransform {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl ViewportTransform {
    /// Convert screen coordinates to canvas coordinates.
    pub fn screen_to_canvas(&self, screen: Position) -> Position {
        Position {
            x: (screen.x - self.pan_x) / self.zoom,
            y: (screen.y - self.pan_y) / self.zoom,
        }
    }

    /// Convert canvas coordinates to screen coordinates.
    pub fn canvas_to_screen(&self, canvas: Position) -> Position {
        Position {
            x: canvas.x * self.zoom + self.pan_x,
            y: canvas.y * self.zoom + self.pan_y,
        }
    }
}

/// State of an in-progress connection being dragged.
#[derive(Clone, Debug)]
pub struct DraftConnection<P: PortId, T: PortType> {
    pub source_port: P,
    pub source_position: Position,
    pub port_type: T,
    pub current_end: Position,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/types.rs
git commit -m "feat: add core types, traits, and geometry primitives"
```

---

## Task 3: Geometry Utilities

**Files:**
- Create: `crates/leptos-node-graph/src/utils.rs`

- [ ] **Step 1: Write geometry utilities**

Replace `crates/leptos-node-graph/src/utils.rs` with:

```rust
use crate::types::Position;

/// Compute a cubic bezier curve control points for a horizontal connection.
/// Source is on the right side of a node, target is on the left.
pub fn bezier_control_points(start: Position, end: Position) -> (Position, Position) {
    let dx = (end.x - start.x).abs();
    let offset = dx.max(50.0) * 0.5;

    let cp1 = Position::new(start.x + offset, start.y);
    let cp2 = Position::new(end.x - offset, end.y);
    (cp1, cp2)
}

/// Generate an SVG path string for a cubic bezier connection.
pub fn bezier_path(start: Position, end: Position) -> String {
    let (cp1, cp2) = bezier_control_points(start, end);
    format!(
        "M {},{} C {},{} {},{} {},{}",
        start.x, start.y, cp1.x, cp1.y, cp2.x, cp2.y, end.x, end.y,
    )
}

/// Sample a point on a cubic bezier curve at parameter t (0..1).
pub fn bezier_point(start: Position, end: Position, t: f64) -> Position {
    let (cp1, cp2) = bezier_control_points(start, end);
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    Position {
        x: mt3 * start.x + 3.0 * mt2 * t * cp1.x + 3.0 * mt * t2 * cp2.x + t3 * end.x,
        y: mt3 * start.y + 3.0 * mt2 * t * cp1.y + 3.0 * mt * t2 * cp2.y + t3 * end.y,
    }
}

/// Approximate minimum distance from a point to a bezier curve.
/// Samples the curve at `steps` intervals.
pub fn distance_to_bezier(point: Position, start: Position, end: Position, steps: usize) -> f64 {
    let mut min_dist = f64::MAX;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let curve_point = bezier_point(start, end, t);
        let dx = point.x - curve_point.x;
        let dy = point.y - curve_point.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

/// Snap a position to a grid.
pub fn snap_to_grid(pos: Position, grid_size: f64) -> Position {
    Position {
        x: (pos.x / grid_size).round() * grid_size,
        y: (pos.y / grid_size).round() * grid_size,
    }
}

/// Distance between two positions.
pub fn distance(a: Position, b: Position) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/utils.rs
git commit -m "feat: add geometry utilities for bezier curves, snapping, hit testing"
```

---

## Task 4: Registry (Reactive State Store)

**Files:**
- Create: `crates/leptos-node-graph/src/registry.rs`

- [ ] **Step 1: Write the EditorRegistry**

Replace `crates/leptos-node-graph/src/registry.rs` with:

```rust
use std::collections::{HashMap, HashSet};

use leptos::prelude::*;

use crate::types::*;

/// Entry for a registered node in the editor.
#[derive(Clone, Debug)]
pub struct NodeEntry<N: NodeId> {
    pub id: N,
    pub position: Position,
    pub size: Size,
}

/// Entry for a registered port in the editor.
#[derive(Clone, Debug)]
pub struct PortEntry<N: NodeId, P: PortId, T: PortType> {
    pub id: P,
    pub node_id: N,
    pub direction: PortDirection,
    pub port_type: T,
    /// Absolute position in canvas space (computed from node position + offset).
    pub position: Position,
}

/// A connection between two ports.
#[derive(Clone, Debug)]
pub struct ConnectionEntry<P: PortId, C: ConnectionId> {
    pub id: C,
    pub source: P,
    pub target: P,
}

/// The central reactive state store for the editor.
/// Stored in context and accessed by all editor components.
#[derive(Clone)]
pub struct EditorRegistry<N, P, C, T>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    /// Registered nodes.
    pub nodes: RwSignal<HashMap<N, NodeEntry<N>>>,
    /// Registered ports.
    pub ports: RwSignal<HashMap<P, PortEntry<N, P, T>>>,
    /// Active connections (provided by consumer).
    pub connections: RwSignal<HashMap<C, ConnectionEntry<P, C>>>,
    /// Currently selected node IDs.
    pub selected_nodes: RwSignal<HashSet<N>>,
    /// Currently selected connection IDs.
    pub selected_connections: RwSignal<HashSet<C>>,
    /// In-progress connection being dragged.
    pub draft_connection: RwSignal<Option<DraftConnection<P, T>>>,
    /// Viewport transform (pan + zoom).
    pub viewport: RwSignal<ViewportTransform>,
    /// Editor configuration.
    pub config: RwSignal<EditorConfig>,
    /// Event callback — sends events to consumer.
    pub on_event: StoredValue<Callback<GraphEvent<N, P, C>>>,
    /// Whether a box select is in progress.
    pub box_select: RwSignal<Option<BoxSelect>>,
    /// Active drag state.
    pub drag_state: RwSignal<Option<DragState<N>>>,
}

/// Box select state.
#[derive(Clone, Debug)]
pub struct BoxSelect {
    pub start: Position,
    pub current: Position,
}

impl BoxSelect {
    pub fn to_rect(&self) -> Rect {
        let x = self.start.x.min(self.current.x);
        let y = self.start.y.min(self.current.y);
        let w = (self.start.x - self.current.x).abs();
        let h = (self.start.y - self.current.y).abs();
        Rect::new(x, y, w, h)
    }
}

/// Drag state for node dragging.
#[derive(Clone, Debug)]
pub struct DragState<N: NodeId> {
    /// The node being dragged (primary).
    pub node_id: N,
    /// Offset from node position to mouse at drag start.
    pub offset: Position,
    /// Starting positions of all dragged nodes (for multi-select drag).
    pub start_positions: HashMap<N, Position>,
}

impl<N, P, C, T> EditorRegistry<N, P, C, T>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
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

    /// Emit a graph event to the consumer.
    pub fn emit(&self, event: GraphEvent<N, P, C>) {
        self.on_event.with_value(|cb| cb.run(event));
    }

    /// Register a node. Called by <Node> on mount.
    pub fn register_node(&self, id: N, position: Position) {
        self.nodes.update(|nodes| {
            nodes.insert(
                id.clone(),
                NodeEntry {
                    id,
                    position,
                    size: Size::default(),
                },
            );
        });
    }

    /// Deregister a node. Called by <Node> on cleanup.
    /// Also removes all ports belonging to this node and their connections.
    pub fn deregister_node(&self, id: &N) {
        // Find ports belonging to this node.
        let port_ids: Vec<P> = self.ports.with_untracked(|ports| {
            ports
                .values()
                .filter(|p| &p.node_id == id)
                .map(|p| p.id.clone())
                .collect()
        });

        // Deregister each port (which handles connection cleanup).
        for port_id in &port_ids {
            self.deregister_port(port_id);
        }

        self.nodes.update(|nodes| {
            nodes.remove(id);
        });

        self.selected_nodes.update(|sel| {
            sel.remove(id);
        });
    }

    /// Register a port. Called by <InputAnchor> / <OutputAnchor> on mount.
    pub fn register_port(&self, id: P, node_id: N, direction: PortDirection, port_type: T, position: Position) {
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

    /// Deregister a port. Called by <InputAnchor> / <OutputAnchor> on cleanup.
    /// Removes any connections referencing this port and emits ConnectionRemoved events.
    pub fn deregister_port(&self, id: &P) {
        let removed_connections: Vec<(C, ConnectionEntry<P, C>)> =
            self.connections.with_untracked(|conns| {
                conns
                    .iter()
                    .filter(|(_, c)| &c.source == id || &c.target == id)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            });

        for (conn_id, _) in &removed_connections {
            self.connections.update(|conns| {
                conns.remove(conn_id);
            });
            self.emit(GraphEvent::ConnectionRemoved {
                id: conn_id.clone(),
            });
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

    /// Update a node's size.
    pub fn set_node_size(&self, id: &N, size: Size) {
        self.nodes.update(|nodes| {
            if let Some(entry) = nodes.get_mut(id) {
                entry.size = size;
            }
        });
    }

    /// Update a port's absolute position.
    pub fn set_port_position(&self, id: &P, position: Position) {
        self.ports.update(|ports| {
            if let Some(entry) = ports.get_mut(id) {
                entry.position = position;
            }
        });
    }

    /// Set connections from consumer data.
    pub fn set_connections(&self, connections: HashMap<C, ConnectionEntry<P, C>>) {
        self.connections.set(connections);
    }

    /// Get the position of a port by ID.
    pub fn port_position(&self, id: &P) -> Option<Position> {
        self.ports.with_untracked(|ports| ports.get(id).map(|p| p.position))
    }

    /// Get port entry by ID.
    pub fn get_port(&self, id: &P) -> Option<PortEntry<N, P, T>> {
        self.ports.with_untracked(|ports| ports.get(id).cloned())
    }

    /// Check if a port is compatible with the current draft connection source.
    pub fn is_compatible_target(&self, target_id: &P) -> bool {
        self.draft_connection.with_untracked(|draft| {
            let Some(draft) = draft.as_ref() else {
                return false;
            };
            self.ports.with_untracked(|ports| {
                let Some(target) = ports.get(target_id) else {
                    return false;
                };
                // Must be an input port.
                if target.direction != PortDirection::Input {
                    return false;
                }
                // Must not be on the same node as the source.
                let Some(source) = ports.get(&draft.source_port) else {
                    return false;
                };
                if source.node_id == target.node_id {
                    return false;
                }
                T::compatible(&draft.port_type, &target.port_type)
            })
        })
    }

    /// Select a single node, deselecting everything else.
    pub fn select_node(&self, id: N) {
        self.selected_connections.set(HashSet::new());
        let mut set = HashSet::new();
        set.insert(id);
        self.selected_nodes.set(set);
        self.emit_selection_changed();
    }

    /// Toggle selection of a node (for shift+click).
    pub fn toggle_node_selection(&self, id: N) {
        self.selected_connections.set(HashSet::new());
        self.selected_nodes.update(|sel| {
            if sel.contains(&id) {
                sel.remove(&id);
            } else {
                sel.insert(id);
            }
        });
        self.emit_selection_changed();
    }

    /// Select a single connection, deselecting everything else.
    pub fn select_connection(&self, id: C) {
        self.selected_nodes.set(HashSet::new());
        let mut set = HashSet::new();
        set.insert(id);
        self.selected_connections.set(set);
        self.emit_selection_changed();
    }

    /// Clear all selection.
    pub fn clear_selection(&self) {
        self.selected_nodes.set(HashSet::new());
        self.selected_connections.set(HashSet::new());
        self.emit_selection_changed();
    }

    /// Select all nodes.
    pub fn select_all(&self) {
        let all_ids: HashSet<N> =
            self.nodes.with_untracked(|nodes| nodes.keys().cloned().collect());
        self.selected_nodes.set(all_ids);
        self.emit_selection_changed();
    }

    fn emit_selection_changed(&self) {
        let nodes = self.selected_nodes.get_untracked();
        let connections = self.selected_connections.get_untracked();
        self.emit(GraphEvent::SelectionChanged { nodes, connections });
    }

    /// Delete all currently selected nodes and connections.
    pub fn delete_selected(&self) {
        let node_ids: Vec<N> = self
            .selected_nodes
            .get_untracked()
            .into_iter()
            .collect();
        let conn_ids: Vec<C> = self
            .selected_connections
            .get_untracked()
            .into_iter()
            .collect();

        if !conn_ids.is_empty() {
            for id in &conn_ids {
                self.emit(GraphEvent::ConnectionRemoved { id: id.clone() });
            }
        }

        if !node_ids.is_empty() {
            self.emit(GraphEvent::NodesDeleted {
                ids: node_ids,
            });
        }

        self.clear_selection();
    }

    /// Get nodes within a rectangle (for box select).
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/registry.rs
git commit -m "feat: add EditorRegistry reactive state store"
```

---

## Task 5: Undo/Redo History

**Files:**
- Create: `crates/leptos-node-graph/src/history.rs`

- [ ] **Step 1: Write the undo/redo stack**

Replace `crates/leptos-node-graph/src/history.rs` with:

```rust
use leptos::prelude::*;

/// A command that can be undone and redone.
/// The library stores opaque command data — the consumer interprets it.
#[derive(Clone, Debug)]
pub struct HistoryCommand<T: Clone + 'static> {
    /// Data representing this command (consumer-defined).
    pub data: T,
}

/// Undo/redo history stack.
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

    /// Push a command onto the undo stack. Clears the redo stack.
    pub fn push(&self, data: T) {
        self.redo_stack.set(Vec::new());
        self.undo_stack.update(|stack| {
            if stack.len() >= self.max_size {
                stack.remove(0);
            }
            stack.push(HistoryCommand { data });
        });
    }

    /// Pop the last command from the undo stack and push it onto the redo stack.
    /// Returns the command data if available.
    pub fn undo(&self) -> Option<T> {
        let cmd = self.undo_stack.try_update(|stack| stack.pop()).flatten();
        if let Some(ref cmd) = cmd {
            self.redo_stack.update(|stack| {
                stack.push(HistoryCommand {
                    data: cmd.data.clone(),
                });
            });
        }
        cmd.map(|c| c.data)
    }

    /// Pop the last command from the redo stack and push it onto the undo stack.
    /// Returns the command data if available.
    pub fn redo(&self) -> Option<T> {
        let cmd = self.redo_stack.try_update(|stack| stack.pop()).flatten();
        if let Some(ref cmd) = cmd {
            self.undo_stack.update(|stack| {
                stack.push(HistoryCommand {
                    data: cmd.data.clone(),
                });
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/history.rs
git commit -m "feat: add undo/redo history stack"
```

---

## Task 6: Layout Trait

**Files:**
- Create: `crates/leptos-node-graph/src/layout.rs`

- [ ] **Step 1: Write the layout trait and types**

Replace `crates/leptos-node-graph/src/layout.rs` with:

```rust
use std::collections::HashMap;

use crate::types::*;

/// Data passed to a layout engine for computation.
#[derive(Clone, Debug)]
pub struct LayoutGraph<N: NodeId, P: PortId> {
    /// Node IDs and their current sizes.
    pub nodes: HashMap<N, Size>,
    /// Connections as (source_node, target_node) pairs.
    pub edges: Vec<(N, N)>,
    /// Port-to-node mapping.
    pub port_to_node: HashMap<P, N>,
}

/// Trait for layout computation. Consumer provides implementations.
pub trait LayoutEngine<N: NodeId, P: PortId> {
    /// Compute new positions for all nodes.
    fn compute(&self, graph: &LayoutGraph<N, P>) -> HashMap<N, Position>;
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/layout.rs
git commit -m "feat: add LayoutEngine trait and LayoutGraph types"
```

---

## Task 7: `<NodeEditor>` Component

**Files:**
- Modify: `crates/leptos-node-graph/src/editor.rs`

- [ ] **Step 1: Write the NodeEditor component**

Replace `crates/leptos-node-graph/src/editor.rs` with:

```rust
use std::collections::HashMap;

use leptos::prelude::*;
use web_sys::MouseEvent;
use web_sys::WheelEvent;
use web_sys::KeyboardEvent;

use crate::connection::ConnectionRenderer;
use crate::interaction;
use crate::registry::{ConnectionEntry, EditorRegistry};
use crate::selection::SelectionBox;
use crate::types::*;

/// The root node graph editor component.
/// Provides editor context to all children, renders canvas layers.
#[component]
pub fn NodeEditor<N, P, C, T>(
    /// Editor configuration.
    #[prop(into)]
    config: EditorConfig,
    /// Reactive connections from the consumer.
    #[prop(into)]
    connections: Signal<HashMap<C, ConnectionEntry<P, C>>>,
    /// Callback for graph events.
    #[prop(into)]
    on_event: Callback<GraphEvent<N, P, C>>,
    /// Child content (nodes).
    children: Children,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = EditorRegistry::<N, P, C, T>::new(config, on_event);
    provide_context(registry.clone());

    // Sync consumer connections into the registry.
    let reg = registry.clone();
    Effect::new(move || {
        let conns = connections.get();
        reg.set_connections(conns);
    });

    let container_ref = NodeRef::<leptos::html::Div>::new();

    // Pan: middle-click drag or Ctrl+left-click drag
    let reg = registry.clone();
    let on_mousedown = move |ev: MouseEvent| {
        interaction::handle_canvas_mousedown(&reg, &ev, container_ref);
    };

    let reg = registry.clone();
    let on_mousemove = move |ev: MouseEvent| {
        interaction::handle_canvas_mousemove(&reg, &ev, container_ref);
    };

    let reg = registry.clone();
    let on_mouseup = move |ev: MouseEvent| {
        interaction::handle_canvas_mouseup(&reg, &ev, container_ref);
    };

    // Zoom: scroll wheel
    let reg = registry.clone();
    let on_wheel = move |ev: WheelEvent| {
        interaction::handle_wheel(&reg, &ev, container_ref);
    };

    // Keyboard shortcuts
    let reg = registry.clone();
    let on_keydown = move |ev: KeyboardEvent| {
        interaction::handle_keydown(&reg, &ev);
    };

    let transform_style = {
        let reg = registry.clone();
        move || {
            let vp = reg.viewport.get();
            format!(
                "transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
                vp.pan_x, vp.pan_y, vp.zoom,
            )
        }
    };

    view! {
        <div
            class="node-editor"
            node_ref=container_ref
            tabindex="0"
            on:mousedown=on_mousedown
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:wheel=on_wheel
            on:keydown=on_keydown
            style="position: relative; overflow: hidden; outline: none;"
        >
            <div
                class="node-editor__canvas"
                style=transform_style
            >
                {children()}
            </div>
            <ConnectionRenderer<N, P, C, T> />
            <SelectionBox<N, P, C, T> />
        </div>
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: may not compile yet — ConnectionRenderer and SelectionBox aren't implemented. That's fine, we'll fill them in over the next tasks. Confirm the error is about missing items in `connection` and `selection` modules.

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/editor.rs
git commit -m "feat: add NodeEditor root component with context provider"
```

---

## Task 8: `<Node>` Component

**Files:**
- Modify: `crates/leptos-node-graph/src/node.rs`

- [ ] **Step 1: Write the Node component**

Replace `crates/leptos-node-graph/src/node.rs` with:

```rust
use leptos::prelude::*;
use web_sys::MouseEvent;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::registry::EditorRegistry;
use crate::types::*;

/// Context provided by a Node to its children (anchors).
#[derive(Clone, Debug)]
pub struct NodeContext<N: NodeId> {
    pub id: N,
    pub position: RwSignal<Position>,
}

/// A node in the graph editor.
/// Registers itself in the registry on mount, deregisters on cleanup.
#[component]
pub fn Node<N, P, C, T>(
    /// Unique node ID.
    id: N,
    /// Reactive node position.
    #[prop(into)]
    position: RwSignal<Position>,
    /// Child content (anchors + node body).
    children: Children,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let node_id = id.clone();

    // Register on mount.
    let reg = registry.clone();
    let register_id = node_id.clone();
    Effect::new(move || {
        let pos = position.get();
        reg.set_node_position(&register_id, pos);
    });

    // Initial registration.
    registry.register_node(node_id.clone(), position.get_untracked());

    // Provide node context for anchors.
    let node_ctx = NodeContext {
        id: node_id.clone(),
        position,
    };
    provide_context(node_ctx);

    let node_ref = NodeRef::<leptos::html::Div>::new();

    // ResizeObserver to track node size.
    let reg = registry.clone();
    let size_id = node_id.clone();
    Effect::new(move || {
        if let Some(el) = node_ref.get() {
            let reg = reg.clone();
            let size_id = size_id.clone();
            let cb = Closure::wrap(Box::new(move |entries: js_sys::Array, _observer: web_sys::ResizeObserver| {
                if let Some(entry) = entries.get(0).dyn_ref::<web_sys::ResizeObserverEntry>() {
                    let rect = entry.content_rect();
                    let size = Size::new(rect.width(), rect.height());
                    reg.set_node_size(&size_id, size);
                }
            }) as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);

            let observer = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref())
                .expect("ResizeObserver should be available");
            observer.observe(&el);

            // Keep closure alive.
            cb.forget();

            on_cleanup(move || {
                observer.disconnect();
            });
        }
    });

    // Deregister on cleanup.
    let reg = registry.clone();
    let cleanup_id = node_id.clone();
    on_cleanup(move || {
        reg.deregister_node(&cleanup_id);
    });

    // Click to select.
    let reg = registry.clone();
    let select_id = node_id.clone();
    let on_mousedown = move |ev: MouseEvent| {
        ev.stop_propagation();

        if ev.shift_key() {
            reg.toggle_node_selection(select_id.clone());
        } else {
            // Start drag.
            let is_selected = reg.selected_nodes.with_untracked(|sel| sel.contains(&select_id));
            if !is_selected {
                reg.select_node(select_id.clone());
            }

            let vp = reg.viewport.get_untracked();
            let canvas_pos = vp.screen_to_canvas(Position::new(ev.client_x() as f64, ev.client_y() as f64));
            let node_pos = reg.nodes.with_untracked(|nodes| {
                nodes.get(&select_id).map(|n| n.position).unwrap_or_default()
            });
            let offset = Position::new(
                canvas_pos.x - node_pos.x,
                canvas_pos.y - node_pos.y,
            );

            // Capture start positions for all selected nodes.
            let start_positions = reg.nodes.with_untracked(|nodes| {
                reg.selected_nodes.with_untracked(|sel| {
                    sel.iter()
                        .filter_map(|id| nodes.get(id).map(|n| (id.clone(), n.position)))
                        .collect()
                })
            });

            reg.drag_state.set(Some(crate::registry::DragState {
                node_id: select_id.clone(),
                offset,
                start_positions,
            }));
        }
    };

    let is_selected = {
        let reg = registry.clone();
        let id = node_id.clone();
        move || reg.selected_nodes.with(|sel| sel.contains(&id))
    };

    let is_dragging = {
        let reg = registry.clone();
        let id = node_id.clone();
        move || {
            reg.drag_state.with(|ds| {
                ds.as_ref().map_or(false, |d| d.node_id == id)
            })
        }
    };

    let style = move || {
        let pos = position.get();
        format!(
            "position: absolute; left: {}px; top: {}px;",
            pos.x, pos.y,
        )
    };

    let class = move || {
        let mut c = String::from("node");
        if is_selected() {
            c.push_str(" node--selected");
        }
        if is_dragging() {
            c.push_str(" node--dragging");
        }
        c
    };

    view! {
        <div
            class=class
            style=style
            node_ref=node_ref
            on:mousedown=on_mousedown
        >
            {children()}
        </div>
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles (or fails only on unimplemented downstream modules)

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/node.rs
git commit -m "feat: add Node component with drag, select, and ResizeObserver"
```

---

## Task 9: `<InputAnchor>` / `<OutputAnchor>` Components

**Files:**
- Modify: `crates/leptos-node-graph/src/anchor.rs`

- [ ] **Step 1: Write the anchor components**

Replace `crates/leptos-node-graph/src/anchor.rs` with:

```rust
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

use crate::node::NodeContext;
use crate::registry::EditorRegistry;
use crate::types::*;

/// An input port on a node.
#[component]
pub fn InputAnchor<N, P, C, T>(
    /// Unique port ID.
    id: P,
    /// Port type for compatibility checking.
    port_type: T,
    /// Optional label.
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Input, label)
}

/// An output port on a node.
#[component]
pub fn OutputAnchor<N, P, C, T>(
    /// Unique port ID.
    id: P,
    /// Port type for compatibility checking.
    port_type: T,
    /// Optional label.
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Output, label)
}

fn anchor_view<N, P, C, T>(
    id: P,
    port_type: T,
    direction: PortDirection,
    label: Option<String>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let node_ctx = expect_context::<NodeContext<N>>();

    let anchor_ref = NodeRef::<leptos::html::Div>::new();
    let port_id = id.clone();

    // Register port with initial position.
    registry.register_port(
        id.clone(),
        node_ctx.id.clone(),
        direction,
        port_type.clone(),
        Position::default(),
    );

    // Update port position when node moves or element changes.
    let reg = registry.clone();
    let pos_port_id = port_id.clone();
    let node_position = node_ctx.position;
    Effect::new(move || {
        // Track node position changes to recompute.
        let _ = node_position.get();
        if let Some(el) = anchor_ref.get() {
            // Get the editor container's bounding rect.
            let el_rect = el.get_bounding_client_rect();

            // Walk up to find the .node-editor container.
            let mut parent = el.parent_element();
            while let Some(p) = &parent {
                if p.class_list().contains("node-editor") {
                    break;
                }
                parent = p.parent_element();
            }

            if let Some(container) = parent {
                let container_rect = container.get_bounding_client_rect();
                let vp = reg.viewport.get_untracked();

                // Compute position in canvas space.
                let screen_x = el_rect.left() + el_rect.width() / 2.0 - container_rect.left();
                let screen_y = el_rect.top() + el_rect.height() / 2.0 - container_rect.top();
                let canvas_pos = vp.screen_to_canvas(Position::new(screen_x, screen_y));

                reg.set_port_position(&pos_port_id, canvas_pos);
            }
        }
    });

    // Deregister on cleanup.
    let reg = registry.clone();
    let cleanup_id = port_id.clone();
    on_cleanup(move || {
        reg.deregister_port(&cleanup_id);
    });

    // Handle mousedown — start or complete a connection.
    let reg = registry.clone();
    let click_port_id = port_id.clone();
    let click_port_type = port_type.clone();
    let on_mousedown = move |ev: MouseEvent| {
        ev.stop_propagation();
        ev.prevent_default();

        match direction {
            PortDirection::Output => {
                // Start a draft connection from this output.
                let pos = reg
                    .port_position(&click_port_id)
                    .unwrap_or_default();
                reg.draft_connection.set(Some(DraftConnection {
                    source_port: click_port_id.clone(),
                    source_position: pos,
                    port_type: click_port_type.clone(),
                    current_end: pos,
                }));
            }
            PortDirection::Input => {
                // Complete a draft connection if one exists and is compatible.
                let should_complete = reg.is_compatible_target(&click_port_id);
                if should_complete {
                    let source = reg.draft_connection.with_untracked(|d| {
                        d.as_ref().map(|d| d.source_port.clone())
                    });
                    if let Some(source) = source {
                        reg.emit(GraphEvent::ConnectionRequested {
                            source,
                            target: click_port_id.clone(),
                        });
                    }
                    reg.draft_connection.set(None);
                }
            }
        }
    };

    let is_compatible = {
        let reg = registry.clone();
        let id = port_id.clone();
        move || {
            if direction != PortDirection::Input {
                return false;
            }
            reg.draft_connection.with(|d| {
                if d.is_none() {
                    return false;
                }
                reg.is_compatible_target(&id)
            })
        }
    };

    let is_connected = {
        let reg = registry.clone();
        let id = port_id.clone();
        move || {
            reg.connections.with(|conns| {
                conns.values().any(|c| c.source == id || c.target == id)
            })
        }
    };

    let dir_class = match direction {
        PortDirection::Input => "anchor--input",
        PortDirection::Output => "anchor--output",
    };

    let class = move || {
        let mut c = format!("anchor {dir_class}");
        if is_compatible() {
            c.push_str(" anchor--compatible");
        }
        if is_connected() {
            c.push_str(" anchor--connected");
        }
        c
    };

    view! {
        <div
            class=class
            node_ref=anchor_ref
            on:mousedown=on_mousedown
        >
            {label.map(|l| view! { <span class="anchor__label">{l}</span> })}
        </div>
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles (or fails only on downstream modules)

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/anchor.rs
git commit -m "feat: add InputAnchor and OutputAnchor components with connection building"
```

---

## Task 10: Connection Renderer

**Files:**
- Modify: `crates/leptos-node-graph/src/connection.rs`

- [ ] **Step 1: Write the ConnectionRenderer component**

Replace `crates/leptos-node-graph/src/connection.rs` with:

```rust
use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::registry::EditorRegistry;
use crate::types::*;
use crate::utils;

/// Renders all connections and the draft connection as SVG paths.
/// This is an internal component rendered by NodeEditor.
#[component]
pub fn ConnectionRenderer<N, P, C, T>() -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();

    let connections_view = {
        let reg = registry.clone();
        move || {
            let conns = reg.connections.get();
            let selected = reg.selected_connections.get();

            conns
                .iter()
                .map(|(id, conn)| {
                    let source_pos = reg.port_position(&conn.source).unwrap_or_default();
                    let target_pos = reg.port_position(&conn.target).unwrap_or_default();
                    let path_d = utils::bezier_path(source_pos, target_pos);
                    let is_selected = selected.contains(id);
                    let class = if is_selected {
                        "connection connection--selected"
                    } else {
                        "connection"
                    };

                    let reg = reg.clone();
                    let conn_id = id.clone();
                    let on_click = move |ev: MouseEvent| {
                        ev.stop_propagation();
                        if ev.shift_key() {
                            reg.selected_connections.update(|sel| {
                                if sel.contains(&conn_id) {
                                    sel.remove(&conn_id);
                                } else {
                                    sel.insert(conn_id.clone());
                                }
                            });
                        } else {
                            reg.select_connection(conn_id.clone());
                        }
                    };

                    view! {
                        <path
                            class=class
                            d=path_d
                            fill="none"
                            stroke-width="2"
                            on:click=on_click
                            style="pointer-events: stroke; cursor: pointer;"
                        />
                    }
                })
                .collect_view()
        }
    };

    let draft_view = {
        let reg = registry.clone();
        move || {
            reg.draft_connection.get().map(|draft| {
                let path_d = utils::bezier_path(draft.source_position, draft.current_end);
                view! {
                    <path
                        class="connection connection--draft"
                        d=path_d
                        fill="none"
                        stroke-width="2"
                        stroke-dasharray="5,5"
                        style="pointer-events: none;"
                    />
                }
            })
        }
    };

    let svg_style = {
        let reg = registry.clone();
        move || {
            let vp = reg.viewport.get();
            format!(
                "position: absolute; top: 0; left: 0; width: 100%; height: 100%; \
                 pointer-events: none; overflow: visible; \
                 transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
                vp.pan_x, vp.pan_y, vp.zoom,
            )
        }
    };

    view! {
        <svg
            class="node-editor__connections"
            style=svg_style
        >
            {connections_view}
            {draft_view}
        </svg>
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles (or fails only on SelectionBox)

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/connection.rs
git commit -m "feat: add ConnectionRenderer with SVG bezier paths and draft connection"
```

---

## Task 11: Selection Box

**Files:**
- Modify: `crates/leptos-node-graph/src/selection.rs`

- [ ] **Step 1: Write the SelectionBox component**

Replace `crates/leptos-node-graph/src/selection.rs` with:

```rust
use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::types::*;

/// Renders the rubber-band box select overlay.
/// Internal component rendered by NodeEditor.
#[component]
pub fn SelectionBox<N, P, C, T>() -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();

    let box_view = move || {
        registry.box_select.get().map(|bs| {
            let rect = bs.to_rect();
            let vp = registry.viewport.get();

            // Convert canvas rect to screen rect for overlay.
            let screen_pos = vp.canvas_to_screen(rect.position);

            let style = format!(
                "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                 pointer-events: none;",
                screen_pos.x,
                screen_pos.y,
                rect.size.width * vp.zoom,
                rect.size.height * vp.zoom,
            );

            view! {
                <div class="selection-box" style=style />
            }
        })
    };

    view! {
        {box_view}
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles (or fails only on interaction module)

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/selection.rs
git commit -m "feat: add SelectionBox overlay component"
```

---

## Task 12: Interaction Handlers

**Files:**
- Modify: `crates/leptos-node-graph/src/interaction.rs`

- [ ] **Step 1: Write interaction handlers**

Replace `crates/leptos-node-graph/src/interaction.rs` with:

```rust
use leptos::prelude::*;
use web_sys::{KeyboardEvent, MouseEvent, WheelEvent};

use crate::registry::{BoxSelect, EditorRegistry};
use crate::types::*;
use crate::utils;

/// Handle mousedown on the editor canvas (not on a node).
pub fn handle_canvas_mousedown<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &MouseEvent,
    container_ref: NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let button = ev.button();

    // Middle-click or Ctrl+left-click: start panning (handled in mousemove).
    if button == 1 || (button == 0 && ev.ctrl_key()) {
        // Panning is handled by tracking mouse delta in mousemove.
        return;
    }

    // Left-click on empty canvas: start box select or clear selection.
    if button == 0 {
        // Cancel any draft connection.
        registry.draft_connection.set(None);

        if !ev.shift_key() {
            registry.clear_selection();
        }

        // Start box select.
        if let Some(container) = container_ref.get_untracked() {
            let rect = container.get_bounding_client_rect();
            let screen_x = ev.client_x() as f64 - rect.left();
            let screen_y = ev.client_y() as f64 - rect.top();
            let vp = registry.viewport.get_untracked();
            let canvas_pos = vp.screen_to_canvas(Position::new(screen_x, screen_y));

            registry.box_select.set(Some(BoxSelect {
                start: canvas_pos,
                current: canvas_pos,
            }));
        }
    }
}

/// Handle mousemove on the editor canvas.
pub fn handle_canvas_mousemove<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &MouseEvent,
    container_ref: NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let buttons = ev.buttons();
    let container = match container_ref.get_untracked() {
        Some(c) => c,
        None => return,
    };
    let container_rect = container.get_bounding_client_rect();
    let screen_x = ev.client_x() as f64 - container_rect.left();
    let screen_y = ev.client_y() as f64 - container_rect.top();

    // Panning: middle-click (button 4 in buttons bitmask) or Ctrl+left (button 1).
    if buttons & 4 != 0 || (buttons & 1 != 0 && ev.ctrl_key()) {
        let dx = ev.movement_x() as f64;
        let dy = ev.movement_y() as f64;
        registry.viewport.update(|vp| {
            vp.pan_x += dx;
            vp.pan_y += dy;
        });
        return;
    }

    let vp = registry.viewport.get_untracked();
    let canvas_pos = vp.screen_to_canvas(Position::new(screen_x, screen_y));

    // Node dragging.
    if buttons & 1 != 0 {
        let drag = registry.drag_state.get_untracked();
        if let Some(drag) = drag {
            let new_pos = Position::new(
                canvas_pos.x - drag.offset.x,
                canvas_pos.y - drag.offset.y,
            );

            // Apply snap-to-grid if configured.
            let config = registry.config.get_untracked();
            let snapped = match config.grid_size {
                Some(grid) => utils::snap_to_grid(new_pos, grid),
                None => new_pos,
            };

            let delta = Position::new(
                snapped.x - drag.start_positions.get(&drag.node_id).map(|p| p.x).unwrap_or(0.0),
                snapped.y - drag.start_positions.get(&drag.node_id).map(|p| p.y).unwrap_or(0.0),
            );

            // Move all selected nodes by the same delta.
            for (id, start_pos) in &drag.start_positions {
                let new = Position::new(start_pos.x + delta.x, start_pos.y + delta.y);
                registry.set_node_position(id, new);
            }
            return;
        }
    }

    // Box select: update current position.
    if buttons & 1 != 0 {
        let has_box = registry.box_select.with_untracked(|bs| bs.is_some());
        if has_box {
            registry.box_select.update(|bs| {
                if let Some(bs) = bs.as_mut() {
                    bs.current = canvas_pos;
                }
            });
            // Update selection to match nodes in rect.
            let rect = registry
                .box_select
                .with_untracked(|bs| bs.as_ref().map(|b| b.to_rect()));
            if let Some(rect) = rect {
                let nodes = registry.nodes_in_rect(&rect);
                registry.selected_nodes.set(nodes);
            }
            return;
        }
    }

    // Draft connection: update end position.
    let has_draft = registry.draft_connection.with_untracked(|d| d.is_some());
    if has_draft {
        registry.draft_connection.update(|d| {
            if let Some(d) = d.as_mut() {
                d.current_end = canvas_pos;
            }
        });
    }
}

/// Handle mouseup on the editor canvas.
pub fn handle_canvas_mouseup<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &MouseEvent,
    _container_ref: NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let _ = ev;

    // End box select.
    if registry.box_select.with_untracked(|bs| bs.is_some()) {
        registry.box_select.set(None);
    }

    // End node drag — emit NodeMoved events.
    if let Some(drag) = registry.drag_state.get_untracked() {
        for (id, start_pos) in &drag.start_positions {
            let current_pos = registry
                .nodes
                .with_untracked(|nodes| nodes.get(id).map(|n| n.position).unwrap_or_default());
            if current_pos != *start_pos {
                registry.emit(GraphEvent::NodeMoved {
                    id: id.clone(),
                    position: current_pos,
                });
            }
        }
        registry.drag_state.set(None);
    }

    // Cancel draft connection if released on empty space.
    registry.draft_connection.set(None);
}

/// Handle scroll wheel for zoom.
pub fn handle_wheel<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &WheelEvent,
    container_ref: NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    ev.prevent_default();

    let container = match container_ref.get_untracked() {
        Some(c) => c,
        None => return,
    };
    let container_rect = container.get_bounding_client_rect();

    let mouse_x = ev.client_x() as f64 - container_rect.left();
    let mouse_y = ev.client_y() as f64 - container_rect.top();

    let delta = -ev.delta_y() * 0.001;
    let config = registry.config.get_untracked();

    registry.viewport.update(|vp| {
        let old_zoom = vp.zoom;
        let new_zoom = (old_zoom * (1.0 + delta)).clamp(config.min_zoom, config.max_zoom);

        // Zoom toward mouse position.
        let scale_change = new_zoom / old_zoom;
        vp.pan_x = mouse_x - (mouse_x - vp.pan_x) * scale_change;
        vp.pan_y = mouse_y - (mouse_y - vp.pan_y) * scale_change;
        vp.zoom = new_zoom;
    });
}

/// Handle keyboard shortcuts.
pub fn handle_keydown<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &KeyboardEvent,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let key = ev.key();
    let ctrl = ev.ctrl_key() || ev.meta_key();
    let shift = ev.shift_key();

    match key.as_str() {
        "Delete" | "Backspace" => {
            ev.prevent_default();
            registry.delete_selected();
        }
        "a" if ctrl => {
            ev.prevent_default();
            registry.select_all();
        }
        "c" if ctrl => {
            ev.prevent_default();
            let ids: Vec<N> = registry
                .selected_nodes
                .get_untracked()
                .into_iter()
                .collect();
            if !ids.is_empty() {
                registry.emit(GraphEvent::NodesCopied { ids });
            }
        }
        "v" if ctrl => {
            ev.prevent_default();
            registry.emit(GraphEvent::NodesPasted {
                offset: Position::new(20.0, 20.0),
            });
        }
        "z" if ctrl && shift => {
            ev.prevent_default();
            registry.emit(GraphEvent::Redo);
        }
        "z" if ctrl => {
            ev.prevent_default();
            registry.emit(GraphEvent::Undo);
        }
        "g" if ctrl => {
            ev.prevent_default();
            let ids: Vec<N> = registry
                .selected_nodes
                .get_untracked()
                .into_iter()
                .collect();
            if ids.len() > 1 {
                registry.emit(GraphEvent::GroupCreated { node_ids: ids });
            }
        }
        "Escape" => {
            registry.draft_connection.set(None);
            registry.clear_selection();
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/interaction.rs
git commit -m "feat: add interaction handlers for pan, zoom, drag, select, keyboard"
```

---

## Task 13: Update lib.rs Exports

**Files:**
- Modify: `crates/leptos-node-graph/src/lib.rs`

- [ ] **Step 1: Update lib.rs with proper exports**

Replace `crates/leptos-node-graph/src/lib.rs` with:

```rust
pub mod types;
pub mod registry;
pub mod editor;
pub mod node;
pub mod anchor;
pub mod connection;
pub mod selection;
pub mod interaction;
pub mod history;
pub mod layout;
pub mod utils;

pub use types::*;
pub use editor::NodeEditor;
pub use node::{Node, NodeContext};
pub use anchor::{InputAnchor, OutputAnchor};
pub use layout::{LayoutEngine, LayoutGraph};
pub use history::UndoHistory;
pub use registry::{EditorRegistry, ConnectionEntry};
```

- [ ] **Step 2: Verify entire library compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles with no errors (warnings okay)

- [ ] **Step 3: Commit**

```bash
git add crates/leptos-node-graph/src/lib.rs
git commit -m "feat: update lib.rs with public API exports"
```

---

## Task 14: Demo App

**Files:**
- Modify: `examples/demo/src/main.rs`
- Modify: `examples/demo/index.html`

- [ ] **Step 1: Write the demo app with sample node types and styling**

Replace `examples/demo/src/main.rs` with:

```rust
use std::collections::HashMap;

use leptos::prelude::*;
use leptos_node_graph::*;
use leptos_node_graph::registry::ConnectionEntry;

/// Our demo port types.
#[derive(Clone, Debug, PartialEq)]
enum DemoPort {
    Float,
    Color,
    Any,
}

impl PortType for DemoPort {
    fn compatible(source: &Self, target: &Self) -> bool {
        matches!(target, DemoPort::Any) || source == target
    }
}

/// A node definition in our demo graph.
#[derive(Clone, Debug)]
struct DemoNode {
    id: String,
    label: String,
    position: RwSignal<Position>,
    inputs: Vec<DemoPortDef>,
    outputs: Vec<DemoPortDef>,
}

/// A port definition.
#[derive(Clone, Debug)]
struct DemoPortDef {
    id: String,
    label: String,
    port_type: DemoPort,
}

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let nodes = RwSignal::new(vec![
        DemoNode {
            id: "node-1".to_string(),
            label: "Color Source".to_string(),
            position: RwSignal::new(Position::new(50.0, 100.0)),
            inputs: vec![],
            outputs: vec![
                DemoPortDef {
                    id: "n1-out-color".to_string(),
                    label: "Color".to_string(),
                    port_type: DemoPort::Color,
                },
                DemoPortDef {
                    id: "n1-out-alpha".to_string(),
                    label: "Alpha".to_string(),
                    port_type: DemoPort::Float,
                },
            ],
        },
        DemoNode {
            id: "node-2".to_string(),
            label: "Math".to_string(),
            position: RwSignal::new(Position::new(350.0, 50.0)),
            inputs: vec![
                DemoPortDef {
                    id: "n2-in-a".to_string(),
                    label: "A".to_string(),
                    port_type: DemoPort::Float,
                },
                DemoPortDef {
                    id: "n2-in-b".to_string(),
                    label: "B".to_string(),
                    port_type: DemoPort::Float,
                },
            ],
            outputs: vec![DemoPortDef {
                id: "n2-out-result".to_string(),
                label: "Result".to_string(),
                port_type: DemoPort::Float,
            }],
        },
        DemoNode {
            id: "node-3".to_string(),
            label: "Output".to_string(),
            position: RwSignal::new(Position::new(650.0, 100.0)),
            inputs: vec![
                DemoPortDef {
                    id: "n3-in-color".to_string(),
                    label: "Color".to_string(),
                    port_type: DemoPort::Color,
                },
                DemoPortDef {
                    id: "n3-in-value".to_string(),
                    label: "Value".to_string(),
                    port_type: DemoPort::Any,
                },
            ],
            outputs: vec![],
        },
    ]);

    let connections: RwSignal<HashMap<String, ConnectionEntry<String, String>>> =
        RwSignal::new(HashMap::new());

    let next_conn_id = StoredValue::new(std::cell::Cell::new(0u64));

    let on_event = Callback::new(move |event: GraphEvent<String, String, String>| {
        match event {
            GraphEvent::NodeMoved { id, position } => {
                nodes.with_untracked(|nodes| {
                    if let Some(node) = nodes.iter().find(|n| n.id == id) {
                        node.position.set(position);
                    }
                });
            }
            GraphEvent::ConnectionRequested { source, target } => {
                let id = next_conn_id.with_value(|cell| {
                    let val = cell.get();
                    cell.set(val + 1);
                    format!("conn-{val}")
                });
                connections.update(|conns| {
                    conns.insert(
                        id.clone(),
                        ConnectionEntry {
                            id,
                            source,
                            target,
                        },
                    );
                });
            }
            GraphEvent::ConnectionRemoved { id } => {
                connections.update(|conns| {
                    conns.remove(&id);
                });
            }
            GraphEvent::NodesDeleted { ids } => {
                nodes.update(|nodes| {
                    nodes.retain(|n| !ids.contains(&n.id));
                });
            }
            _ => {
                web_sys::console::log_1(
                    &format!("Event: {:?}", event).into(),
                );
            }
        }
    });

    let config = EditorConfig::default();

    view! {
        <style>
            {r#"
                html, body {
                    margin: 0;
                    padding: 0;
                    height: 100%;
                    background: #1a1a2e;
                    font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
                    color: #e0e0e0;
                }
                .node-editor {
                    width: 100vw;
                    height: 100vh;
                    background:
                        radial-gradient(circle at center, #16213e 0%, #1a1a2e 100%),
                        repeating-linear-gradient(0deg, transparent, transparent 19px, #ffffff08 19px, #ffffff08 20px),
                        repeating-linear-gradient(90deg, transparent, transparent 19px, #ffffff08 19px, #ffffff08 20px);
                    cursor: grab;
                }
                .node-editor:active { cursor: grabbing; }
                .node {
                    background: #16213e;
                    border: 1px solid #0f3460;
                    border-radius: 8px;
                    min-width: 160px;
                    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                    cursor: default;
                    user-select: none;
                }
                .node--selected {
                    border-color: #e94560;
                    box-shadow: 0 0 0 2px #e9456040, 0 4px 12px rgba(0,0,0,0.3);
                }
                .node--dragging { opacity: 0.9; }
                .node__header {
                    padding: 8px 12px;
                    background: #0f3460;
                    border-radius: 7px 7px 0 0;
                    font-size: 12px;
                    font-weight: 600;
                    text-transform: uppercase;
                    letter-spacing: 0.5px;
                }
                .node__ports {
                    padding: 8px 0;
                }
                .node__port-row {
                    display: flex;
                    align-items: center;
                    padding: 4px 12px;
                    gap: 8px;
                }
                .node__port-row--input { justify-content: flex-start; }
                .node__port-row--output { justify-content: flex-end; }
                .anchor {
                    width: 12px;
                    height: 12px;
                    border-radius: 50%;
                    border: 2px solid #533483;
                    background: #1a1a2e;
                    cursor: crosshair;
                    flex-shrink: 0;
                    transition: all 0.15s ease;
                }
                .anchor:hover {
                    border-color: #e94560;
                    transform: scale(1.3);
                }
                .anchor--compatible {
                    border-color: #00d2ff;
                    background: #00d2ff30;
                    animation: pulse 1s infinite;
                }
                .anchor--connected { background: #533483; }
                .anchor__label {
                    font-size: 11px;
                    color: #a0a0b0;
                }
                .connection { stroke: #533483; transition: stroke 0.15s; }
                .connection--selected { stroke: #e94560; stroke-width: 3; }
                .connection--draft { stroke: #00d2ff; opacity: 0.7; }
                .selection-box {
                    background: #e9456015;
                    border: 1px solid #e94560;
                    border-radius: 2px;
                }
                @keyframes pulse {
                    0%, 100% { box-shadow: 0 0 0 0 #00d2ff40; }
                    50% { box-shadow: 0 0 0 4px #00d2ff20; }
                }
            "#}
        </style>
        <NodeEditor<String, String, String, DemoPort>
            config=config
            connections=Signal::derive(move || connections.get())
            on_event=on_event
        >
            {move || {
                nodes.get().into_iter().map(|node| {
                    let node_id = node.id.clone();
                    let label = node.label.clone();
                    let inputs = node.inputs.clone();
                    let outputs = node.outputs.clone();
                    let position = node.position;

                    view! {
                        <Node<String, String, String, DemoPort>
                            id=node_id
                            position=position
                        >
                            <div class="node__header">{label}</div>
                            <div class="node__ports">
                                {inputs.into_iter().map(|port| {
                                    view! {
                                        <div class="node__port-row node__port-row--input">
                                            <InputAnchor<String, String, String, DemoPort>
                                                id=port.id
                                                port_type=port.port_type
                                                label=port.label
                                            />
                                        </div>
                                    }
                                }).collect_view()}
                                {outputs.into_iter().map(|port| {
                                    view! {
                                        <div class="node__port-row node__port-row--output">
                                            <OutputAnchor<String, String, String, DemoPort>
                                                id=port.id
                                                port_type=port.port_type
                                                label=port.label
                                            />
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </Node<String, String, String, DemoPort>>
                    }
                }).collect_view()
            }}
        </NodeEditor<String, String, String, DemoPort>>
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Test with Trunk**

Run: `cd /home/trevor/Code/leptos-node-graph/examples/demo && trunk serve`
Expected: compiles to WASM and serves at localhost. Browser shows three nodes with ports. Pan/zoom works. Dragging nodes works. Creating connections by clicking output then input anchors works. Box select works. Delete key removes selected items.

- [ ] **Step 4: Commit**

```bash
git add examples/demo/src/main.rs
git commit -m "feat: add demo app with sample nodes, connections, and styling"
```

---

## Task 15: Compile Fix Pass

This task exists because earlier tasks were written incrementally and may have minor compilation issues (missing imports, trait bound mismatches, Leptos API differences from the researched patterns vs actual 0.7 API). This task fixes all compilation errors.

**Files:**
- Modify: any files with compilation errors

- [ ] **Step 1: Run cargo check and fix all errors**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check 2>&1`

Read each error. Fix the source file. Common expected issues:
- Leptos 0.7 `view!` macro syntax for generics (may need turbofish or different syntax)
- `StoredValue` API may differ in 0.7
- `Callback` may need `Callback::new` or different `#[prop(into)]` handling
- `on_cleanup` import path
- SVG element rendering in `view!` macro

Fix each error, re-run `cargo check` until clean.

- [ ] **Step 2: Run cargo check on the demo too**

Run: `cd /home/trevor/Code/leptos-node-graph && cargo check --workspace 2>&1`
Expected: entire workspace compiles

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "fix: resolve compilation errors across library and demo"
```

---

## Task 16: Smoke Test and Polish

- [ ] **Step 1: Run the demo with Trunk**

Run: `cd /home/trevor/Code/leptos-node-graph/examples/demo && trunk serve`
Open browser to the URL shown.

- [ ] **Step 2: Verify core interactions**

Test each of these in the browser:
1. Three nodes visible with ports
2. Pan by middle-click drag or Ctrl+drag
3. Zoom with scroll wheel (zoom toward cursor)
4. Click a node to select it (red border)
5. Shift+click to multi-select
6. Drag a node to move it
7. Drag on empty space to box select
8. Click an output anchor, then an input anchor to create a connection
9. Click a connection to select it
10. Delete key removes selected nodes/connections
11. Ctrl+A selects all
12. Escape clears selection

- [ ] **Step 3: Fix any runtime issues found**

Address any runtime bugs. Common issues: coordinate transforms, event propagation, SVG positioning.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: runtime polish from smoke testing"
```

---

## Task 17: Add .gitignore

**Files:**
- Create: `.gitignore`

- [ ] **Step 1: Write .gitignore**

```
/target
dist/
.superpowers/
```

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git commit -m "chore: add .gitignore"
```
