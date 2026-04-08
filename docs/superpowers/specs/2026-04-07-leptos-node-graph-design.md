# leptos-node-graph Design Spec

## Overview

A Leptos component library providing a headless, extensible node graph editor framework. The library owns all interaction logic (connections, dragging, selection, pan/zoom, copy/paste, undo/redo, keyboard shortcuts, grouping) but leaves visual styling and node content entirely to the consumer.

**Key design principles:**
- Headless/unstyled — zero default visual styles, full CSS control for consumers
- Trait-driven extensibility — port types, layout engines, and rendering are all consumer-defined
- Hybrid state — library owns transient UI state, consumer owns graph data
- Fully dynamic — nodes and ports can be added/removed/changed at runtime from remote data

## Data Model

The library defines traits rather than concrete types. The consumer implements them for their domain.

### Core Traits

```rust
/// Consumer implements this to define port type compatibility
pub trait PortType: Clone + PartialEq + 'static {
    fn compatible(source: &Self, target: &Self) -> bool;
}

/// Identity traits — consumer provides these (UUIDs, strings, integers, etc.)
pub trait NodeId: Clone + Eq + Hash + 'static {}
pub trait PortId: Clone + Eq + Hash + 'static {}
pub trait ConnectionId: Clone + Eq + Hash + 'static {}
```

### Internal Registry

The library maintains an internal reactive registry tracking:

- **Node entries** — id, position (x, y), size (w, h), registered input/output ports
- **Port entries** — id, parent node, direction (input/output), port type, computed position relative to node
- **Connections** — id, source port, target port
- **Transient UI state** — selection set, active drag, in-progress draft connection, viewport transform, undo/redo history stack

### Dynamic Registration

Nodes and ports register on mount and deregister on cleanup. When a port is deregistered, any connections referencing it are automatically removed (emitting `ConnectionRemoved` events). Same cascading behavior for node removal.

Port definitions on existing nodes can change at runtime — the registry tracks the current set reactively.

## Component Architecture

### `<NodeEditor<T: PortType>>`

Root component. Sets up editor context, renders the canvas layers, handles global keyboard events.

**Props:**
- `graph` — reactive signal of the consumer's graph data
- `on_event` — callback receiving `GraphEvent` mutations
- `config` — `EditorConfig` (snap-to-grid, zoom limits, shortcut bindings, layout mode)

**Renders:**
- An HTML container with CSS transform for pan/zoom
- An SVG overlay for connection paths
- A slot for children (consumer's nodes)

### `<Node>`

Wraps a single node. Registers in the registry on mount, deregisters on cleanup.

**Props:**
- `id` — node identifier
- `position` — reactive (x, y) signal
- Optional `size`

**Behavior:**
- Provides its own nested context so child anchors know their parent node
- Handles drag-to-move, click-to-select
- Observes its own size via ResizeObserver and reports to registry

### `<InputAnchor<T>>` / `<OutputAnchor<T>>`

A single port. Registers in the registry on mount, deregisters (with connection cascade cleanup) on cleanup.

**Props:**
- `id` — port identifier
- `port_type` — the consumer's `PortType` value
- Optional `label`

**Behavior:**
- Click/drag to start or complete a connection
- Computes its position relative to the parent node and reports to registry
- Library calls `PortType::compatible` during hover to determine if a connection is valid

### Internal Components (not consumer-facing)

- **`<ConnectionRenderer>`** — reads all connections from the registry and renders SVG bezier paths. Also renders the in-progress draft connection while dragging.
- **`<SelectionBox>`** — renders the rubber-band box select overlay during drag-select.

### Consumer Usage Pattern

```rust
<NodeEditor<MyPortType>
    graph=graph_signal
    on_event=handle_event
    config=editor_config
>
    <For each=move || nodes.get()
        key=|n| n.id
        let:node
    >
        <Node id=node.id position=node.position>
            <For each=move || node.inputs()
                key=|p| p.id
                let:port
            >
                <InputAnchor id=port.id port_type=port.kind />
            </For>
            <For each=move || node.outputs()
                key=|p| p.id
                let:port
            >
                <OutputAnchor id=port.id port_type=port.kind />
            </For>
            // Consumer renders whatever they want here
            {(node.render_body)()}
        </Node>
    </For>
</NodeEditor>
```

## Event System

The library never mutates the consumer's graph data directly. All mutations are communicated via a `GraphEvent` enum:

```rust
pub enum GraphEvent<N: NodeId, P: PortId, C: ConnectionId> {
    NodeMoved { id: N, position: (f64, f64) },
    NodeResized { id: N, size: (f64, f64) },
    ConnectionRequested { source: P, target: P },
    ConnectionRemoved { id: C },
    SelectionChanged { nodes: HashSet<N>, connections: HashSet<C> },
    NodesDeleted { ids: Vec<N> },
    NodesCopied { ids: Vec<N> },
    NodesPasted { offset: (f64, f64) },
    Undo,
    Redo,
    GroupCreated { node_ids: Vec<N> },
}
```

The consumer applies these events to their own data store. The library's internal registry stays in sync via the reactive graph signal.

## Interaction System

### Pan & Zoom
- Middle-click drag or Ctrl+drag to pan
- Scroll wheel to zoom (configurable min/max limits)
- Single CSS transform on the canvas container (GPU-accelerated)
- Viewport state tracked in registry for spatial queries/culling

### Node Dragging
- Left-click drag on a node moves it (or entire selection if multi-selected)
- Optional snap-to-grid (configurable grid size)
- Emits `NodeMoved` on drag end (not continuously)

### Connection Building
- Mousedown on an output anchor starts a draft connection
- Hovering over a compatible input anchor highlights it (via `PortType::compatible`)
- Mouseup on compatible anchor emits `ConnectionCreated`
- Mouseup on empty space or incompatible anchor cancels
- Also supports click-to-start, click-to-complete flow

### Selection
- Click to select (deselects others)
- Shift+click for multi-select toggle
- Drag on empty canvas for box select
- Ctrl+A for select all
- Click on connection SVG path to select connections

### Keyboard Shortcuts (all configurable via `EditorConfig`)
- Delete/Backspace — remove selected nodes/connections
- Ctrl+C/V — copy/paste (paste at mouse position with offset)
- Ctrl+Z / Ctrl+Shift+Z — undo/redo
- Ctrl+G — group selected nodes

### Undo/Redo
- Command pattern — each mutation is a reversible command pushed onto a history stack
- Library tracks history internally
- Emits `Undo`/`Redo` events so consumer can reverse their own state in sync

## Layout System

### Layout Trait

```rust
pub trait LayoutEngine<N: NodeId> {
    fn compute(&self, graph: &LayoutGraph<N>) -> HashMap<N, (f64, f64)>;
}
```

### Classic Mode (default)
- Free-positioned nodes on an infinite canvas
- No layout engine required
- Consumer can optionally provide a `LayoutEngine` impl for auto-layout (triggered via API call)

### Structured Mode
- Nodes snap to columns/slots based on connection topology
- Library provides grid computation and slot management
- Positioning algorithm comes from the consumer's `LayoutEngine` impl
- Mode switching is a config change — library handles position re-sync when toggling

## Rendering & Styling

### Rendering Approach
- **HTML nodes** — regular DOM elements, consumers can put arbitrary Leptos components inside
- **SVG connections** — resolution-independent bezier curves, crisp at any zoom, easy hit-testing
- **CSS transform pan/zoom** — single transform on canvas container, GPU-accelerated

### CSS Classes (no default styles applied)
- `.node-editor` — root container
- `.node-editor__canvas` — transformed canvas layer
- `.node-editor__connections` — SVG connection overlay
- `.node` / `.node--selected` / `.node--dragging`
- `.anchor` / `.anchor--input` / `.anchor--output` / `.anchor--compatible` / `.anchor--connected`
- `.connection` / `.connection--selected` / `.connection--draft`
- `.selection-box`

Consumers can also use render props/slot patterns for full control over anchor and connection rendering when CSS alone isn't sufficient.

## Project Structure

```
leptos-node-graph/
├── Cargo.toml                  # workspace
├── crates/
│   └── leptos-node-graph/
│       ├── Cargo.toml          # library crate
│       └── src/
│           ├── lib.rs
│           ├── editor.rs       # <NodeEditor> component + context
│           ├── node.rs         # <Node> component
│           ├── anchor.rs       # <InputAnchor> / <OutputAnchor>
│           ├── connection.rs   # connection rendering (SVG)
│           ├── selection.rs    # selection box + selection state
│           ├── registry.rs     # internal reactive registry
│           ├── interaction.rs  # drag, pan/zoom, keyboard handlers
│           ├── history.rs      # undo/redo command stack
│           ├── layout.rs       # layout trait + mode management
│           ├── types.rs        # traits, enums, config types
│           └── utils.rs        # geometry helpers
└── examples/
    └── demo/
        ├── Cargo.toml          # trunk app
        ├── index.html          # trunk entry point
        └── src/
            └── main.rs         # demo app with sample node types
```

## Demo App

The `examples/demo/` Trunk app demonstrates:
- Several sample node types with different port configurations
- Dynamic node/port creation and removal
- Connection building with type checking
- All interaction features (pan, zoom, drag, select, box select, delete, copy/paste, undo/redo)
- Custom CSS styling showing the headless nature of the library
- Both classic and structured layout modes
