/// Style configuration for the node creation menu.
#[derive(Clone, Debug)]
pub struct NodeMenuStyle {
    /// Menu panel background.
    pub background: String,
    /// Menu panel border.
    pub border: String,
    /// Menu panel box shadow.
    pub shadow: String,
    /// Search input background.
    pub input_background: String,
    /// Search input border.
    pub input_border: String,
    /// Search input text color.
    pub input_color: String,
    /// Placeholder text color.
    pub placeholder_color: String,
    /// Category header text color.
    pub category_color: String,
    /// Menu item text color.
    pub item_color: String,
    /// Menu item description color.
    pub description_color: String,
    /// Hover/selected background.
    pub hover_background: String,
    /// Port sub-item text color.
    pub port_color: String,
    /// "No matching nodes" text color.
    pub empty_color: String,
    /// Divider/border between sections.
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
    /// Default group color when none specified.
    pub default_color: String,
    /// Border radius.
    pub border_radius: String,
    /// Error state border color.
    pub error_border: String,
    /// Error state background.
    pub error_background: String,
    /// Error state label color.
    pub error_label_color: String,
    /// Label font size.
    pub label_font_size: String,
    /// Label font weight.
    pub label_font_weight: String,
    /// Rename input border color (uses group color by default, this is fallback).
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
