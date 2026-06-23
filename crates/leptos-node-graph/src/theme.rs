/// Controls how a node's input and output anchors are arranged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnchorLayout {
    /// Inputs in a left column, outputs in a right column (default).
    #[default]
    Columns,
    /// A single vertical stack: all inputs, then all outputs below them.
    /// Each row spans the full node width so consumers can lay anchor
    /// children out as a `[name | value]` grid. Input dots stay on the left
    /// edge, output dots on the right edge, so connection routing is unchanged.
    Stacked,
}

/// Shape of an anchor's socket dot. Per-anchor override; defaults to `Circle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DotShape {
    /// Round socket (default).
    #[default]
    Circle,
    /// Rotated square — e.g. for vector/diamond sockets.
    Diamond,
    /// Square socket.
    Square,
}

/// Style configuration for node cards.
#[derive(Clone, Debug)]
pub struct NodeStyle {
    pub background: String,
    pub border: String,
    /// Selection outline (uses CSS outline, not border — no layout shift).
    pub outline_selected: String,
    pub border_radius: String,
    pub shadow: String,
    pub shadow_selected: String,
    pub min_width: String,
    /// Fixed node width (CSS length, e.g. "256px"). When non-empty the node
    /// renders at exactly this width; combined with the node's `overflow:
    /// hidden`, body field content that doesn't fit elides (the field value
    /// spans are `text-overflow: ellipsis`). Empty = size to content above
    /// `min_width` (the prior behavior).
    pub width: String,
    pub opacity_dragging: f64,
    /// Horizontal padding applied to all sections (header, body, ports).
    pub padding_x: f64,
    /// Vertical padding for the header section.
    pub header_padding_y: f64,
    /// Vertical padding for the body section (0 if no body).
    pub body_padding_y: f64,
    /// Vertical padding for the ports section.
    pub ports_padding_y: f64,
    /// Border between header and body/ports.
    pub header_border_bottom: String,
    /// Background for the header section.
    pub header_background: String,
    /// Header text color.
    pub header_color: String,
    /// Header font size.
    pub header_font_size: String,
    /// Accent line height in pixels (used when Node has accent_color prop).
    pub header_accent_height: f64,
    /// Body field label color.
    pub field_label_color: String,
    /// Body field label font size.
    pub field_label_font_size: String,
    /// Body field gap between label and content.
    pub field_gap: String,
    /// Body field label min width.
    pub field_label_min_width: String,
    /// Body border bottom (between body and ports).
    pub body_border_bottom: String,
    /// How input and output anchors are arranged within the node.
    pub anchor_layout: AnchorLayout,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            background: "#1e1e22".into(),
            border: "1px solid #3f3f46".into(),
            outline_selected: "1.5px solid #ef4444".into(),
            border_radius: "8px".into(),
            shadow: "0 4px 12px rgba(0,0,0,0.4)".into(),
            shadow_selected: "0 0 0 1px #ef4444, 0 4px 16px rgba(239,68,68,0.25)".into(),
            min_width: "160px".into(),
            width: String::new(),
            opacity_dragging: 0.92,
            padding_x: 10.0,
            header_padding_y: 6.0,
            body_padding_y: 6.0,
            ports_padding_y: 4.0,
            header_border_bottom: "1px solid #27272a".into(),
            header_background: "#232327".into(),
            header_color: "#a1a1aa".into(),
            header_font_size: "12px".into(),
            header_accent_height: 2.0,
            field_label_color: "#71717a".into(),
            field_label_font_size: "10px".into(),
            field_gap: "6px".into(),
            field_label_min_width: "38px".into(),
            body_border_bottom: "1px solid #27272a".into(),
            anchor_layout: AnchorLayout::Columns,
        }
    }
}

/// Style configuration for the node creation menu.
#[derive(Clone, Debug)]
pub struct NodeMenuStyle {
    pub background: String,
    pub border: String,
    pub shadow: String,
    pub input_background: String,
    pub input_border: String,
    pub input_color: String,
    pub placeholder_color: String,
    pub category_color: String,
    pub item_color: String,
    pub description_color: String,
    pub hover_background: String,
    pub port_color: String,
    pub empty_color: String,
    pub divider: String,
}

impl Default for NodeMenuStyle {
    fn default() -> Self {
        Self {
            background: "#1e1e22".into(),
            border: "1px solid #3f3f46".into(),
            shadow: "0 8px 24px rgba(0,0,0,0.5)".into(),
            input_background: "#27272a".into(),
            input_border: "1px solid #3f3f46".into(),
            input_color: "#d4d4d8".into(),
            placeholder_color: "#71717a".into(),
            category_color: "#52525b".into(),
            item_color: "#d4d4d8".into(),
            description_color: "#71717a".into(),
            hover_background: "rgba(99, 102, 241, 0.15)".into(),
            port_color: "#a1a1aa".into(),
            empty_color: "#71717a".into(),
            divider: "1px solid #27272a".into(),
        }
    }
}

/// Style configuration for group box overlays.
#[derive(Clone, Debug)]
pub struct GroupStyle {
    pub default_color: String,
    pub border_radius: String,
    pub error_border: String,
    pub error_background: String,
    pub error_label_color: String,
    pub label_font_size: String,
    pub label_font_weight: String,
    pub input_background: String,
}

impl Default for GroupStyle {
    fn default() -> Self {
        Self {
            default_color: "#8b5cf6".into(),
            border_radius: "8px".into(),
            error_border: "rgba(239, 68, 68, 0.5)".into(),
            error_background: "rgba(239, 68, 68, 0.08)".into(),
            error_label_color: "rgba(239, 68, 68, 0.8)".into(),
            label_font_size: "10px".into(),
            label_font_weight: "600".into(),
            input_background: "transparent".into(),
        }
    }
}

/// Style configuration for anchor (port) rendering.
/// The library renders the full anchor row: dot + label/children.
#[derive(Clone, Debug)]
pub struct AnchorStyle {
    /// Dot size in pixels.
    pub dot_size: f64,
    /// Dot border width.
    pub dot_border_width: f64,
    /// Dot default border color (unconnected, no draft).
    pub dot_color: String,
    /// Dot color when connected.
    pub dot_connected_color: String,
    /// Dot color when compatible with draft.
    pub dot_compatible_color: String,
    /// Dot glow when compatible.
    pub dot_compatible_shadow: String,
    /// Label font size.
    pub label_font_size: String,
    /// Label default color.
    pub label_color: String,
    /// Label color when compatible.
    pub label_compatible_color: String,
    /// Fixed row height in pixels. Children cannot exceed this.
    pub row_height: f64,
    /// Y offset from node top to the center of the first port.
    /// Typically header_height + padding + row_height/2.
    pub first_port_y: f64,
    /// X inset from node edge for the dot center (input ports from left, output from right).
    pub dot_inset: f64,
    /// Row padding (CSS).
    pub row_padding: String,
    /// Row gap between dot and label.
    pub row_gap: String,
    /// Opacity when incompatible during draft.
    pub incompatible_opacity: f64,
    /// Tooltip background.
    pub tooltip_background: String,
    /// Tooltip border.
    pub tooltip_border: String,
    /// Tooltip text color.
    pub tooltip_color: String,
}

impl Default for AnchorStyle {
    fn default() -> Self {
        Self {
            dot_size: 8.0,
            dot_border_width: 1.5,
            dot_color: "#71717a".into(),
            dot_connected_color: "#71717a".into(),
            dot_compatible_color: "#22d3ee".into(),
            dot_compatible_shadow: "0 0 6px #22d3ee, 0 0 12px rgba(34,211,238,0.3)".into(),
            label_font_size: "11px".into(),
            label_color: "#a1a1aa".into(),
            label_compatible_color: "#22d3ee".into(),
            row_height: 24.0,
            first_port_y: 0.0, // computed from header+body measurements
            dot_inset: 14.0,
            row_padding: "0 10px".into(),
            row_gap: "6px".into(),
            incompatible_opacity: 0.25,
            tooltip_background: "#27272a".into(),
            tooltip_border: "1px solid #3f3f46".into(),
            tooltip_color: "#a1a1aa".into(),
        }
    }
}
