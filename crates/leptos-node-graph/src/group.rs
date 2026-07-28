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
}

/// Renders group box overlays. Groups are visual containers around nodes.
///
/// - Double-click a label to rename it inline
/// - Alt+drag a node: immediately removes it from its group.
///   Dropping onto another group adds it there.
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
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let padding = padding.unwrap_or(16.0);

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
                        for group in &current_groups {
                            if let Some(bounds) =
                                compute_bounds(&group.node_ids, &nodes_map, padding)
                                && bounds.contains(center)
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
                    let mut found = None;
                    for group in &current_groups {
                        if let Some(bounds) = compute_bounds(&group.node_ids, &all_nodes, padding)
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
        let nodes = registry.nodes.get();
        let hovered = hover_group.get();

        groups
            .into_iter()
            .filter_map(|group| {
                let bounds = compute_bounds(&group.node_ids, &nodes, padding)?;

                let has_label = group.label.is_some();
                let label_height = if has_label { 24.0 } else { 0.0 };

                let final_bounds = GroupBounds {
                    x: bounds.x,
                    y: bounds.y - label_height,
                    width: bounds.width,
                    height: bounds.height + label_height,
                };

                let gs = use_context::<GroupStyle>().unwrap_or_default();
                let color = group
                    .color
                    .clone()
                    .unwrap_or_else(|| gs.default_color.clone());
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

                let box_style = format!(
                    "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                     pointer-events: none; z-index: 0; \
                     background: {bg_color}; border: 1px {border_style} {border_color}; \
                     border-radius: {}; transition: background 0.15s, border 0.15s;",
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
                        let on_event_rename = on_event;

                        view! {
                            <GroupLabel
                                label=label
                                color=label_color
                                group_id=group_id
                                on_rename=on_event_rename
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

/// Editable group label — double-click to rename.
#[component]
fn GroupLabel<N: NodeId>(
    label: String,
    color: String,
    group_id: String,
    on_rename: Option<Callback<GroupEvent<N>>>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let (text, set_text) = signal(label.clone());
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let gs = use_context::<GroupStyle>().unwrap_or_default();

    let label_style = format!(
        "position: absolute; top: 6px; left: 10px; \
         font-size: {}; font-weight: {}; text-transform: uppercase; \
         letter-spacing: 0.05em; color: {color}; \
         pointer-events: auto; cursor: default;",
        gs.label_font_size, gs.label_font_weight
    );

    let input_style = format!(
        "position: absolute; top: 4px; left: 8px; \
         font-size: 10px; font-weight: 600; text-transform: uppercase; \
         letter-spacing: 0.05em; color: {color}; \
         background: {}; border: 1px solid {color}; \
         border-radius: 3px; padding: 1px 4px; outline: none; \
         pointer-events: auto;",
        gs.input_background
    );

    let group_id_commit = group_id.clone();
    let on_rename_commit = on_rename;
    let commit = move || {
        set_editing.set(false);
        let new_label = text.get_untracked();
        if let Some(ref cb) = on_rename_commit {
            cb.run(GroupEvent::Renamed {
                group_id: group_id_commit.clone(),
                new_label,
            });
        }
    };

    // Focus input when entering edit mode
    Effect::new(move || {
        if editing.get()
            && let Some(el) = input_ref.get()
        {
            let _ = el.focus();
            el.select();
        }
    });

    move || {
        if editing.get() {
            let commit_blur = commit.clone();
            let commit_key = commit.clone();
            view! {
                <input
                    node_ref=input_ref
                    type="text"
                    style=input_style.clone()
                    prop:value=move || text.get()
                    on:input=move |ev| {
                        use leptos::wasm_bindgen::JsCast;
                        let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                        set_text.set(t.value());
                    }
                    on:blur=move |_| { commit_blur(); }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        ev.stop_propagation();
                        if ev.key() == "Enter" || ev.key() == "Escape" {
                            commit_key();
                        }
                    }
                    on:mousedown=move |ev: web_sys::MouseEvent| { ev.stop_propagation(); }
                />
            }
            .into_any()
        } else {
            view! {
                <span
                    style=label_style.clone()
                    on:dblclick=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        set_editing.set(true);
                    }
                >
                    {move || text.get()}
                </span>
            }
            .into_any()
        }
    }
}

/// Compute bounding box from a set of node IDs.
fn compute_bounds<N: NodeId>(
    node_ids: &[N],
    nodes: &std::collections::HashMap<N, crate::registry::NodeEntry<N>>,
    padding: f64,
) -> Option<GroupBounds> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;

    for node_id in node_ids {
        if let Some(entry) = nodes.get(node_id) {
            let x = entry.position.x;
            let y = entry.position.y;
            let w = if entry.size.width > 0.0 {
                entry.size.width
            } else {
                160.0
            };
            let h = if entry.size.height > 0.0 {
                entry.size.height
            } else {
                80.0
            };

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + w);
            max_y = max_y.max(y + h);
            found = true;
        }
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
