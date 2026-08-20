use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::theme::GroupStyle;
use crate::types::*;

/// Defines a visual group box around a set of nodes.
#[derive(Clone, Debug)]
pub struct GroupBox<N: NodeId> {
    /// Unique group identifier.
    pub id: String,
    /// Node IDs contained in this group.
    pub node_ids: Vec<N>,
    /// Optional label displayed above the group.
    pub label: Option<String>,
    /// Group color as a CSS color string (e.g. "#8b5cf6").
    pub color: Option<String>,
    /// Whether the group is in an error state.
    pub error: bool,
}

/// Computed bounding box for a group.
#[derive(Clone, Debug)]
pub struct GroupBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl GroupBounds {
    pub fn contains(&self, point: Position) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }
}

/// Events emitted by the group overlay.
#[derive(Clone, Debug)]
pub enum GroupEvent<N: NodeId> {
    /// Group label was renamed.
    Renamed { group_id: String, new_label: String },
    /// A node was added to a group (alt+drag onto group box).
    NodeAdded { group_id: String, node_id: N },
    /// A node was removed from a group (alt+drag started).
    NodeRemoved { group_id: String, node_id: N },
    /// The group color chip was cycled.
    ColorChanged { group_id: String, new_color: String },
}

/// Renders group box overlays. Groups are visual containers around nodes.
///
/// - Double-click a label to rename it inline
/// - Alt+drag an empty canvas: creates a group from the enclosed nodes.
/// - Alt+drag a node: immediately removes it from its group. Dropping onto
///   another group adds it there.
/// - Ctrl+G: creates a group from the current selection.
#[component]
pub fn GroupBoxOverlay<N, P, C, T>(
    /// Reactive list of group definitions.
    groups: Signal<Vec<GroupBox<N>>>,
    /// Callback for group events (rename, add/remove node).
    #[prop(optional, into)]
    on_event: Option<Callback<GroupEvent<N>>>,
    /// Padding around the group bounding box in pixels.
    #[prop(optional, into)]
    padding: Option<f64>,
    /// PhantomData for unused type parameters.
    #[prop(optional)]
    _marker: PhantomData<(P, C, T)>,
    /// Optional callback for custom header rendering per group.
    #[prop(optional)]
    header: Option<Callback<(GroupBox<N>, GroupBounds), AnyView>>,
    /// Optional action for removing a group.
    #[prop(optional)]
    on_ungroup: Option<Callback<String>>,
    /// Optional action for selecting every member of a group.
    #[prop(optional)]
    on_select_all: Option<Callback<Vec<N>>>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let padding = padding.unwrap_or(16.0);
    // Keep the library's selection state in sync with the consumer callback.
    // Nodes render their highlight from this registry signal; updating only a
    // consumer-owned selection signal makes the action appear to do nothing.
    let on_select_all_nodes = on_select_all.map(|callback| {
        let registry = registry.clone();
        Callback::new(move |node_ids: Vec<N>| {
            registry
                .selected_nodes
                .set(node_ids.iter().cloned().collect());
            callback.run(node_ids);
        })
    });

    // Track which group the alt-dragged node is hovering over (for visual feedback)
    let hover_group: RwSignal<Option<String>> = RwSignal::new(None);
    // Track previous drag info to detect start/end transitions
    let prev_alt_drag: RwSignal<Option<N>> = RwSignal::new(None);

    let reg_drag = registry.clone();
    let on_event_drag = on_event;
    Effect::new(move || {
        let drag = reg_drag.drag_state.get();
        let prev = prev_alt_drag.get_untracked();

        match (&drag, &prev) {
            // Alt-drag just started
            (Some(ds), None) if ds.alt_key => {
                let node_id = ds.node_id.clone();
                prev_alt_drag.set(Some(node_id.clone()));

                // Immediately remove node from all its groups
                if let Some(ref on_ev) = on_event_drag {
                    let current_groups = groups.get_untracked();
                    for group in &current_groups {
                        if group.node_ids.contains(&node_id) {
                            on_ev.run(GroupEvent::NodeRemoved {
                                group_id: group.id.clone(),
                                node_id: node_id.clone(),
                            });
                        }
                    }
                }
            }
            // Non-alt drag started — ignore
            (Some(_), None) => {}
            // Alt-drag ended — check if node is over a group and add it
            (None, Some(node_id)) => {
                prev_alt_drag.set(None);
                hover_group.set(None);

                if let Some(ref on_ev) = on_event_drag {
                    // Get node's final center position
                    let node_center = reg_drag.nodes.with_untracked(|nodes| {
                        nodes.get(node_id).map(|n| {
                            Position::new(
                                n.position.x + n.size.width / 2.0,
                                n.position.y + n.size.height / 2.0,
                            )
                        })
                    });

                    if let Some(center) = node_center {
                        // Check against current group bounds
                        let current_groups = groups.get_untracked();
                        let nodes_map = reg_drag.nodes.get_untracked();
                        let live_positions = reg_drag.live_positions.get_untracked();
                        for group in &current_groups {
                            if let Some(bounds) = compute_bounds(
                                &group.node_ids,
                                &nodes_map,
                                &live_positions,
                                padding,
                            ) && bounds.contains(center)
                            {
                                on_ev.run(GroupEvent::NodeAdded {
                                    group_id: group.id.clone(),
                                    node_id: node_id.clone(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Update hover highlight during alt-drag.
        // We read nodes reactively here so this re-runs as the node moves.
        if let Some(ref ds) = drag {
            if ds.alt_key {
                let dragged_id = ds.node_id.clone();
                let all_nodes = reg_drag.nodes.get(); // reactive — triggers on position change
                let node_center = all_nodes.get(&dragged_id).map(|n| {
                    Position::new(
                        n.position.x + n.size.width / 2.0,
                        n.position.y + n.size.height / 2.0,
                    )
                });

                if let Some(center) = node_center {
                    let current_groups = groups.get_untracked();
                    let live_positions = reg_drag.live_positions.get();
                    let mut found = None;
                    for group in &current_groups {
                        if let Some(bounds) =
                            compute_bounds(&group.node_ids, &all_nodes, &live_positions, padding)
                            && bounds.contains(center)
                        {
                            found = Some(group.id.clone());
                            break;
                        }
                    }
                    hover_group.set(found);
                }
            }
        } else {
            hover_group.set(None);
        }
    });

    // Render groups
    move || {
        let groups = groups.get();
        // `compute_bounds` reads each node's live consumer-owned position
        // signal, while this map read tracks membership and measured sizes.
        let nodes = registry.nodes.get();
        let live_positions = registry.live_positions.get();
        let hovered = hover_group.get();

        groups
            .into_iter()
            .filter_map(|group| {
                let bounds = compute_bounds(&group.node_ids, &nodes, &live_positions, padding)?;

                let has_label = group.label.is_some();
                let header_height = if has_label { 24.0 } else { padding };

                let final_bounds = GroupBounds {
                    x: bounds.x,
                    // `bounds` already reserves `padding` above the first node.
                    // The header replaces that top padding instead of stacking
                    // another row above it.
                    y: bounds.y + padding - header_height,
                    width: bounds.width,
                    height: bounds.height + header_height - padding,
                };

                let gs = use_context::<GroupStyle>().unwrap_or_default();
                let color = group
                    .color
                    .clone()
                    .unwrap_or_else(|| gs.default_color.clone());
                // The original slate default is too bright as a translucent
                // group fill; normalize it for existing persisted groups too.
                let color = if color.eq_ignore_ascii_case("#64748b") {
                    "#3f4752".to_string()
                } else {
                    color
                };
                let is_error = group.error;
                let is_hovered = hovered.as_ref() == Some(&group.id);

                let (border_color, bg_color) = if is_error {
                    (gs.error_border.clone(), gs.error_background.clone())
                } else if is_hovered {
                    (
                        format!("color-mix(in srgb, {color} 80%, transparent)"),
                        format!("color-mix(in srgb, {color} 20%, transparent)"),
                    )
                } else {
                    (
                        format!("color-mix(in srgb, {color} 50%, transparent)"),
                        format!("color-mix(in srgb, {color} 10%, transparent)"),
                    )
                };

                let border_style = if is_hovered { "solid" } else { "dashed" };
                let glow = if is_hovered {
                    format!(
                        "box-shadow: 0 0 0 2px color-mix(in srgb, {color} 70%, transparent), 0 0 22px color-mix(in srgb, {color} 45%, transparent);"
                    )
                } else {
                    String::new()
                };

                let box_style = format!(
                    "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                     pointer-events: none; z-index: 0; \
                     background: {bg_color}; border: 1px {border_style} {border_color}; \
                     border-radius: {}; {glow} transition: background 0.15s, border 0.15s, box-shadow 0.15s;",
                    final_bounds.x,
                    final_bounds.y,
                    final_bounds.width,
                    final_bounds.height,
                    gs.border_radius,
                );

                let header_view = if let Some(ref header_cb) = header {
                    Some(header_cb.run((group.clone(), final_bounds.clone())))
                } else {
                    group.label.clone().map(|label| {
                        let label_color = if is_error {
                            gs.error_label_color.clone()
                        } else {
                            format!("color-mix(in srgb, {color} 70%, white)")
                        };

                        let group_id = group.id.clone();
                        let group_color = color.clone();
                        let on_event_rename = on_event;

                        view! {
                            <GroupLabel
                                label=label
                                color=label_color
                                group_color=group_color
                                group_id=group_id
                                on_rename=on_event_rename
                                on_ungroup=on_ungroup
                                on_select_all=on_select_all_nodes
                                node_ids=group.node_ids.clone()
                            />
                        }
                        .into_any()
                    })
                };

                Some(view! {
                    <div style=box_style role="presentation">
                        {header_view}
                    </div>
                })
            })
            .collect_view()
    }
}

/// Editable group label.
#[component]
fn GroupLabel<N: NodeId>(
    label: String,
    color: String,
    group_color: String,
    group_id: String,
    on_rename: Option<Callback<GroupEvent<N>>>,
    on_ungroup: Option<Callback<String>>,
    on_select_all: Option<Callback<Vec<N>>>,
    node_ids: Vec<N>,
) -> impl IntoView {
    let (text, set_text) = signal(label.clone());

    let gs = use_context::<GroupStyle>().unwrap_or_default();

    let header_style = Signal::derive(move || {
        format!(
            "position:absolute; top:4px; left:10px; right:4px; z-index:2; height:16px; \
         display:flex; align-items:center; gap:6px; min-width:0; \
         font-size: {}; font-weight: {}; text-transform: uppercase; \
         letter-spacing:0.05em; color:{}; background:transparent; \
         line-height:16px; white-space:nowrap; \
         pointer-events: auto; cursor: default;",
            gs.label_font_size, gs.label_font_weight, color
        )
    });

    let input_style = "flex:1 1 auto; min-width:0; height:18px; margin:0; padding:0 4px; \
         box-sizing:border-box; border:0; outline:none; border-radius:3px; \
         background:transparent; color:inherit; font:inherit; line-height:18px; \
         text-transform:uppercase; letter-spacing:0.05em; pointer-events:auto;";

    let group_id_commit = group_id.clone();
    let on_rename_commit = on_rename;
    let on_color = on_rename;
    let commit = move || {
        let new_label = text.get_untracked();
        if let Some(ref cb) = on_rename_commit {
            cb.run(GroupEvent::Renamed {
                group_id: group_id_commit.clone(),
                new_label,
            });
        }
    };

    let commit_blur = commit.clone();
    let commit_key = commit.clone();
    view! {
        <div
            style=header_style
            title="Ctrl+G with nodes selected creates a group. Alt-drag removes this node; drop onto another group to add it."
            on:mousedown=|ev: web_sys::MouseEvent| ev.stop_propagation()
            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
        >
            <button
                type="button"
                title="Change group color"
                aria-label="Change group color"
                style=format!("flex:0 0 10px; width:10px; height:10px; margin:0; padding:0; border:0; border-radius:50%; background:{}; cursor:pointer; pointer-events:auto;", group_color)
                on:mousedown=|ev: web_sys::MouseEvent| ev.stop_propagation()
                on:dblclick=|ev: web_sys::MouseEvent| ev.stop_propagation()
                on:click={
                    let group_id = group_id.clone();
                    let current = group_color.clone();
                    move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        if let Some(ref callback) = on_color {
                            callback.run(GroupEvent::ColorChanged {
                                group_id: group_id.clone(),
                                new_color: next_group_color(&current),
                            });
                        }
                    }
                }
            ></button>
            <input
                type="text"
                style=input_style
                prop:value=move || text.get()
                aria-label="Group title"
                on:input=move |ev| {
                    use leptos::wasm_bindgen::JsCast;
                    let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                    set_text.set(t.value());
                }
                on:blur=move |_| { commit_blur(); }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    ev.stop_propagation();
                    if ev.key() == "Enter" {
                        commit_key();
                    } else if ev.key() == "Escape" {
                        set_text.set(label.clone());
                        ev.prevent_default();
                    }
                }
                on:mousedown=|ev: web_sys::MouseEvent| ev.stop_propagation()
            />
            <span style="display:flex; flex:0 0 auto; align-items:center; gap:6px; height:16px;">
                    {on_select_all.map(|callback| {
                        let node_ids = node_ids.clone();
                        view! {
                            <button
                                type="button"
                                title="Select all in group"
                                style="display:inline-flex; align-items:center; justify-content:center; width:16px; height:16px; margin:0; padding:0; pointer-events:auto; cursor:pointer; background:transparent; border:0; color:inherit; opacity:0.7;"
                                on:mousedown=|ev: web_sys::MouseEvent| ev.stop_propagation()
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    callback.run(node_ids.clone());
                                }
                            >
                                <svg viewBox="0 0 24 24" width="12" height="12" aria-hidden="true"><path fill="currentColor" d="M4 4h4v4H4V4m6 0h4v4h-4V4m6 0h4v4h-4V4M4 10h4v4H4v-4m6 0h4v4h-4v-4m6 0h4v4h-4v-4M4 16h4v4H4v-4m6 0h4v4h-4v-4m6 0h4v4h-4v-4Z"/></svg>
                            </button>
                        }
                    })}
                    {on_ungroup.map(|callback| {
                        let group_id = group_id.clone();
                        view! {
                            <button
                                type="button"
                                title="Ungroup"
                                style="display:inline-flex; align-items:center; justify-content:center; width:16px; height:16px; margin:0; padding:0; pointer-events:auto; cursor:pointer; background:transparent; border:0; color:inherit; opacity:0.7;"
                                on:mousedown=|ev: web_sys::MouseEvent| ev.stop_propagation()
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    callback.run(group_id.clone());
                                }
                            >
                                <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path fill="currentColor" d="m18.3 5.71-1.41-1.42L12 9.17 7.11 4.29 5.7 5.71l4.89 4.88-4.89 4.89 1.41 1.41L12 12l4.89 4.89 1.41-1.41-4.89-4.89 4.89-4.88Z"/></svg>
                            </button>
                        }
                    })}
            </span>
        </div>
    }
}

fn next_group_color(current: &str) -> String {
    const COLORS: [&str; 7] = [
        "#3f4752", "#EF476F", "#F78C6B", "#FFD166", "#06D6A0", "#118AB2", "#073B4C",
    ];
    let index = COLORS
        .iter()
        .position(|color| *color == current)
        .unwrap_or(0);
    COLORS[(index + 1) % COLORS.len()].to_string()
}

/// Compute bounding box from a set of node IDs.
fn compute_bounds<N: NodeId>(
    node_ids: &[N],
    nodes: &std::collections::HashMap<N, crate::registry::NodeEntry<N>>,
    live_positions: &std::collections::HashMap<N, Position>,
    padding: f64,
) -> Option<GroupBounds> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;

    for node_id in node_ids {
        let Some(entry) = nodes.get(node_id) else {
            // A group with a stale membership should not render a misleading
            // box. The consumer can remove the missing node and the group will
            // recover on the next reactive update.
            return None;
        };
        // Use a conservative fallback until the node's first measurement
        // arrives. The overlay must exist during that initial frame; measured
        // dimensions replace these values reactively as soon as they land.
        let w = entry.size.width.max(160.0);
        let h = entry.size.height.max(80.0);
        // The registry position is the live interaction position. The
        // consumer-owned signal may be backed by resolved/persisted state and
        // only catch up when the drag is committed, so using it here makes
        // group bounds update only on drop.
        let position = live_positions
            .get(node_id)
            .copied()
            .unwrap_or(entry.position);
        let x = position.x;
        let y = position.y;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
        found = true;
    }

    if !found {
        return None;
    }

    Some(GroupBounds {
        x: min_x - padding,
        y: min_y - padding,
        width: (max_x - min_x) + padding * 2.0,
        height: (max_y - min_y) + padding * 2.0,
    })
}
