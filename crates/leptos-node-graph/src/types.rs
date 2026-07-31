use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

/// Consumer implements this to define port type compatibility.
pub trait PortType: Clone + PartialEq + Debug + Send + Sync + 'static {
    /// Returns true if a connection from `source` output to `target` input is valid.
    fn compatible(source: &Self, target: &Self) -> bool;

    /// Returns a string identifier for this port type, used for menu filtering.
    /// Must round-trip with `from_type_id`.
    fn type_id(&self) -> String;

    /// Reconstruct a port type from its string identifier.
    /// Must round-trip with `type_id`.
    fn from_type_id(id: &str) -> Self;

    /// Check compatibility using string type IDs.
    /// Default uses `from_type_id` + `compatible`. Override if you need custom logic.
    fn compatible_by_id(source_id: &str, target_id: &str) -> bool {
        Self::compatible(
            &Self::from_type_id(source_id),
            &Self::from_type_id(target_id),
        )
    }
}

pub trait NodeId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}
pub trait PortId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}
pub trait ConnectionId: Clone + Eq + Hash + Debug + Send + Sync + 'static {}

// Blanket implementations for common types
impl NodeId for String {}
impl PortId for String {}
impl ConnectionId for String {}
impl NodeId for u64 {}
impl PortId for u64 {}
impl ConnectionId for u64 {}
impl NodeId for usize {}
impl PortId for usize {}
impl ConnectionId for usize {}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub enum GraphEvent<N, P, C>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
{
    /// Final positions for every node moved by one drag gesture.
    NodesMoved {
        nodes: Vec<(N, Position)>,
    },
    NodeResized {
        id: N,
        size: Size,
    },
    ConnectionRequested {
        source: P,
        target: P,
    },
    ConnectionRemoved {
        id: C,
    },
    SelectionChanged {
        nodes: HashSet<N>,
        connections: HashSet<C>,
    },
    NodesDeleted {
        ids: Vec<N>,
    },
    NodesCopied {
        ids: Vec<N>,
    },
    NodesPasted {
        offset: Position,
    },
    Undo,
    Redo,
    GroupCreated {
        node_ids: Vec<N>,
    },
    /// User selected a node type from the creation menu.
    /// Consumer should create the node, and if `connect_from`/`connect_to` are set,
    /// also create a connection between them.
    CreateNode {
        /// The menu item id (node type).
        item_id: String,
        /// Canvas position where the node should be created.
        position: Position,
        /// The port that initiated the draft connection (if menu was opened during a drag).
        connect_from: Option<P>,
        /// The port id on the new node to connect to (e.g. "a", "result").
        /// Consumer prefixes this with the new node's id.
        connect_to: Option<String>,
        /// Direction of the draft origin, so consumer knows output→input order.
        connect_direction: Option<PortDirection>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutMode {
    #[default]
    Classic,
    Structured,
}

#[derive(Clone, Debug)]
pub struct EditorConfig {
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub grid_size: Option<f64>,
    pub layout_mode: LayoutMode,
    /// Screen-pixel inset left around the graph when framing it (the `F` key).
    pub fit_padding: f64,
    /// Zoom ceiling when framing. Separate from `max_zoom` so that framing a
    /// small graph fills the pane at a sane size instead of magnifying it —
    /// framing may always zoom OUT as far as `min_zoom`.
    pub fit_max_zoom: f64,
    /// How close (in SCREEN pixels, so the pull feels the same at any zoom) the
    /// cursor must come to a compatible port before a draft connection snaps to
    /// it. `0.0` disables snapping.
    pub snap_distance: f64,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            min_zoom: 0.1,
            max_zoom: 5.0,
            grid_size: None,
            layout_mode: LayoutMode::Classic,
            fit_padding: 48.0,
            fit_max_zoom: 1.0,
            snap_distance: 22.0,
        }
    }
}

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
    pub fn screen_to_canvas(&self, screen: Position) -> Position {
        Position {
            x: (screen.x - self.pan_x) / self.zoom,
            y: (screen.y - self.pan_y) / self.zoom,
        }
    }

    pub fn canvas_to_screen(&self, canvas: Position) -> Position {
        Position {
            x: canvas.x * self.zoom + self.pan_x,
            y: canvas.y * self.zoom + self.pan_y,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DraftConnection<P: PortId, T: PortType> {
    pub source_port: P,
    pub source_position: Position,
    pub port_type: T,
    pub current_end: Position,
    /// The direction of the port where the drag started.
    pub origin_direction: PortDirection,
    /// Compatible port the draft is currently snapped to, if the cursor is
    /// within `EditorConfig::snap_distance` of one. `current_end` sits exactly on
    /// that port while this is set, and releasing completes the connection to it
    /// — so the wire never appears attached to something it wouldn't connect to.
    pub snap_target: Option<P>,
}
