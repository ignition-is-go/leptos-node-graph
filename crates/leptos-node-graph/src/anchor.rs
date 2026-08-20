use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::use_event_listener;

use crate::node::NodeContext;
use crate::registry::EditorRegistry;
use crate::types::*;

/// Context provided by an anchor to its children.
/// Consumers read these signals to drive their own styling.
/// Attach `dot_ref` to the element that represents the port dot — the library
/// uses it to compute connection endpoint positions.
#[derive(Clone)]
pub struct AnchorContext {
    pub direction: PortDirection,
    pub is_compatible: Signal<bool>,
    pub is_incompatible: Signal<bool>,
    pub is_source: Signal<bool>,
    pub is_connected: Signal<bool>,
    pub has_broken_connections: Signal<bool>,
    pub dot_ref: NodeRef<leptos::html::Div>,
    /// The port type identifier (from `PortType::type_id()`).
    pub port_type_label: String,
}

/// Items for the anchor context menu.
#[derive(Clone, Debug)]
pub struct AnchorMenuItem {
    pub label: String,
    pub action: AnchorMenuAction,
    pub enabled: bool,
}

impl AnchorMenuItem {
    /// An enabled item running a consumer callback.
    pub fn action(label: impl Into<String>, on_select: Callback<()>) -> Self {
        Self {
            label: label.into(),
            action: AnchorMenuAction::Custom(on_select),
            enabled: true,
        }
    }
}

#[derive(Clone)]
pub enum AnchorMenuAction {
    /// Remove all connections on this port.
    RemoveConnections,
    /// Remove only broken connections (where the other port is missing).
    RemoveBrokenConnections,
    /// A consumer-supplied action (see [`AnchorMenuBuilder`]). Run by the
    /// anchor, which then closes the menu.
    Custom(Callback<()>),
}

impl std::fmt::Debug for AnchorMenuAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RemoveConnections => f.write_str("RemoveConnections"),
            Self::RemoveBrokenConnections => f.write_str("RemoveBrokenConnections"),
            Self::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}

/// Consumer hook for the anchor right-click menu. `provide_context` one above
/// the editor to REPLACE the built-in "Remove connections / Remove broken
/// connections" items with your own — the consumer is the only side that knows
/// how to name the thing on the other end of a wire.
///
/// The closure runs inside a reactive scope, so reading signals in it keeps the
/// menu live. Returning an empty list means "no menu here": the right-click is
/// swallowed and nothing opens (an anchor with nothing to offer shouldn't
/// flash an empty panel).
#[derive(Clone)]
pub struct AnchorMenuBuilder<P: PortId>(AnchorMenuFn<P>);

/// The builder's boxed closure: `(port, direction) -> items`.
type AnchorMenuFn<P> =
    std::sync::Arc<dyn Fn(&P, PortDirection) -> Vec<AnchorMenuItem> + Send + Sync>;

impl<P: PortId> AnchorMenuBuilder<P> {
    pub fn new(
        build: impl Fn(&P, PortDirection) -> Vec<AnchorMenuItem> + Send + Sync + 'static,
    ) -> Self {
        Self(std::sync::Arc::new(build))
    }

    pub fn build(&self, port: &P, direction: PortDirection) -> Vec<AnchorMenuItem> {
        (self.0)(port, direction)
    }
}

/// State for the anchor context menu, provided to consumer for rendering.
#[derive(Clone)]
pub struct AnchorMenuState {
    /// Screen position to render the menu at.
    pub position: RwSignal<Option<Position>>,
    /// Menu items with current enabled state.
    pub items: Signal<Vec<AnchorMenuItem>>,
    /// Call this to execute an action and close the menu.
    pub on_action: Callback<AnchorMenuAction>,
    /// Call this to close the menu without action.
    pub on_close: Callback<()>,
}

/// Try to complete a draft connection on the given port.
/// Returns true if the connection was completed.
pub(crate) fn try_complete_connection<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    port_id: &P,
    port_direction: PortDirection,
) -> bool
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let draft = registry.draft_connection.with_untracked(|d| d.clone());
    let Some(draft) = draft else { return false };

    // The completing port must be the opposite direction from where the draft started
    if port_direction == draft.origin_direction {
        return false;
    }

    // Check compatibility — need to figure out which is output and which is input
    let target_port = registry.get_port(port_id);
    let source_port_entry = registry.get_port(&draft.source_port);
    let (Some(target), Some(source)) = (target_port, source_port_entry) else {
        return false;
    };

    // Must be on different nodes
    if source.node_id == target.node_id {
        return false;
    }

    // Determine output→input order for compatibility check
    let (output_type, input_type) = if draft.origin_direction == PortDirection::Output {
        (&source.port_type, &target.port_type)
    } else {
        (&target.port_type, &source.port_type)
    };

    if !T::compatible(output_type, input_type) {
        return false;
    }

    // Emit connection with correct output→input direction
    let (out_id, in_id) = if draft.origin_direction == PortDirection::Output {
        (draft.source_port.clone(), port_id.clone())
    } else {
        (port_id.clone(), draft.source_port.clone())
    };

    registry.emit(GraphEvent::ConnectionRequested {
        source: out_id,
        target: in_id,
    });
    registry.draft_connection.set(None);
    true
}

#[allow(clippy::too_many_arguments)]
fn anchor_view<N, P, C, T>(
    id: P,
    port_type: T,
    direction: PortDirection,
    label: Option<String>,
    children: Option<Children>,
    show_type: bool,
    dot_color: Option<String>,
    dot_shape: crate::theme::DotShape,
    dot_multi: bool,
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
    let dot_ref = NodeRef::<leptos::html::Div>::new();

    // Register port
    let initial_pos = node_ctx.position.get_untracked();
    registry.register_port(
        id.clone(),
        node_ctx.id.clone(),
        direction,
        port_type.clone(),
        initial_pos,
    );

    // Deregister on cleanup
    let reg_cleanup = registry.clone();
    let id_cleanup = id.clone();
    on_cleanup(move || {
        reg_cleanup.deregister_port(&id_cleanup);
    });

    // Deterministic port position calculation — no DOM measurement needed.
    // Uses node position, measured header/body heights, slot index, and row height.
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    let anchor_style = use_context::<crate::theme::AnchorStyle>().unwrap_or_default();
    let node_style = use_context::<crate::theme::NodeStyle>().unwrap_or_default();
    let row_h = anchor_style.row_height;
    let dot_inset = anchor_style.dot_inset;
    let ports_pad_top = node_style.ports_padding_y;
    // In Stacked layout, outputs render below the inputs in one column, so an
    // output's row index must include the input count for its Y to line up.
    let stacked = node_style.anchor_layout == crate::theme::AnchorLayout::Stacked;

    Effect::new(move || {
        let node_pos = node_ctx.position.get();
        let ports_y = node_ctx.ports_y_offset.get();
        let nw = node_ctx.node_width.get();

        // Get this port's slot index, plus (in Stacked mode for outputs) the
        // number of input rows that sit above this output column.
        let (slot_idx, rows_above) = reg_pos.ports.with_untracked(|ports| {
            let slot = ports.get(&id_pos).map(|p| p.slot_index).unwrap_or(0);
            let above = if stacked && direction == PortDirection::Output {
                ports
                    .values()
                    .filter(|p| p.node_id == node_ctx.id && p.direction == PortDirection::Input)
                    .count()
            } else {
                0
            };
            (slot, above)
        });
        let row_idx = slot_idx + rows_above;

        let is_dragging = reg_pos.drag_state.with_untracked(|ds| ds.is_some());
        if is_dragging {
            return; // batch_set_positions handles this
        }

        let y = node_pos.y + ports_y + ports_pad_top + (row_idx as f64 * row_h) + (row_h / 2.0);
        let x = match direction {
            PortDirection::Input => node_pos.x + dot_inset,
            PortDirection::Output => node_pos.x + nw - dot_inset,
        };

        let canvas_pos = Position::new(x, y);
        reg_pos.set_port_position(&id_pos, canvas_pos);

        // Store offset for batch_set_positions during drag
        let offset = Position::new(x - node_pos.x, y - node_pos.y);
        reg_pos.set_port_offset(&id_pos, offset);
    });

    // Mousedown: start a draft from any port, or click-complete an existing draft
    let reg_md = registry.clone();
    let id_md = id.clone();
    let pt = port_type.clone();
    let _ = use_event_listener(
        anchor_ref,
        leptos::ev::mousedown,
        move |ev: web_sys::MouseEvent| {
            // Primary button only. A right-click belongs to the anchor's context
            // menu — without this it started a draft (and on a connected input,
            // tore the existing wire down to re-route it) before the menu could
            // ever open.
            if ev.button() != 0 {
                return;
            }

            let has_draft = reg_md.draft_connection.with_untracked(|d| d.is_some());

            if has_draft {
                // A draft is in flight and the press landed on this port. Swallow
                // the event, but DON'T connect here: connections are only ever
                // created on mouseup, so a press that gets dragged away or
                // cancelled never leaves a connection behind. The anchor's own
                // mouseup handler completes it on release.
                //
                // Swallowing matters twice over — it stops the canvas handler
                // from cancelling the draft, and stops the `else` branch below
                // from starting a competing draft from this port.
                ev.stop_propagation();
                ev.prevent_default();
            } else {
                // Start a new draft — only from the dot element
                let on_dot = if let Some(target) = ev.target() {
                    use leptos::wasm_bindgen::JsCast;
                    target
                        .dyn_ref::<web_sys::Element>()
                        .is_some_and(|el| el.closest("[data-anchor-dot]").ok().flatten().is_some())
                } else {
                    false
                };

                if !on_dot {
                    return;
                }

                ev.stop_propagation();
                ev.prevent_default();

                // If this is a connected input, disconnect and re-route from the original output
                if direction == PortDirection::Input {
                    let existing = reg_md.connections.with_untracked(|conns| {
                        conns
                            .iter()
                            .find(|(_, c)| c.target == id_md)
                            .map(|(conn_id, c)| (conn_id.clone(), c.source.clone()))
                    });

                    if let Some((conn_id, source_port_id)) = existing {
                        // Remove the connection
                        reg_md.emit(GraphEvent::ConnectionRemoved { id: conn_id });

                        // Start draft from the original output
                        if let Some(source_entry) = reg_md.get_port(&source_port_id) {
                            let source_pos =
                                reg_md.port_position(&source_port_id).unwrap_or_default();
                            reg_md.draft_connection.set(Some(DraftConnection {
                                source_port: source_port_id,
                                source_position: source_pos,
                                port_type: source_entry.port_type.clone(),
                                current_end: source_pos,
                                origin_direction: PortDirection::Output,
                                snap_target: None,
                            }));
                        }
                        return;
                    }
                }

                // Normal: start a new draft from this port
                let port_pos = reg_md.port_position(&id_md);
                if let Some(pos) = port_pos {
                    reg_md.draft_connection.set(Some(DraftConnection {
                        source_port: id_md.clone(),
                        source_position: pos,
                        port_type: pt.clone(),
                        current_end: pos,
                        origin_direction: direction,
                        snap_target: None,
                    }));
                }
            }
        },
    );

    // Mouseup: complete a drag-to-connect on any port
    let reg_mu = registry.clone();
    let id_mu = id.clone();
    let _ = use_event_listener(
        anchor_ref,
        leptos::ev::mouseup,
        move |ev: web_sys::MouseEvent| {
            // Primary button only — releasing a right-click over a port must not
            // land a connection (see the mousedown handler).
            if ev.button() != 0 {
                return;
            }
            let has_draft = reg_mu.draft_connection.with_untracked(|d| d.is_some());
            if has_draft && try_complete_connection(&reg_mu, &id_mu, direction) {
                ev.stop_propagation();
            }
        },
    );

    // Derived state signals
    // A port is a valid drop target if it's the opposite direction from the draft origin,
    // on a different node, and type-compatible.
    let id_compat = id.clone();
    let reg_compat = registry.clone();
    let is_compatible = Signal::derive(move || {
        reg_compat.draft_connection.with(|d| {
            let Some(d) = d.as_ref() else { return false };
            if direction == d.origin_direction {
                return false;
            }
            if d.source_port == id_compat {
                return false;
            }
            let Some(this_port) = reg_compat.get_port(&id_compat) else {
                return false;
            };
            let Some(draft_port) = reg_compat.get_port(&d.source_port) else {
                return false;
            };
            if this_port.node_id == draft_port.node_id {
                return false;
            }
            let (out_type, in_type) = if d.origin_direction == PortDirection::Output {
                (&draft_port.port_type, &this_port.port_type)
            } else {
                (&this_port.port_type, &draft_port.port_type)
            };
            T::compatible(out_type, in_type)
        })
    });

    let id_incompat = id.clone();
    let reg_incompat = registry.clone();
    let is_incompatible = Signal::derive(move || {
        reg_incompat.draft_connection.with(|d| {
            let Some(d) = d.as_ref() else { return false };
            if d.source_port == id_incompat {
                return false;
            } // source is not incompatible
            if direction == d.origin_direction {
                return true;
            } // same direction = can't connect
            let Some(this_port) = reg_incompat.get_port(&id_incompat) else {
                return true;
            };
            let Some(draft_port) = reg_incompat.get_port(&d.source_port) else {
                return true;
            };
            if this_port.node_id == draft_port.node_id {
                return true;
            }
            let (out_type, in_type) = if d.origin_direction == PortDirection::Output {
                (&draft_port.port_type, &this_port.port_type)
            } else {
                (&this_port.port_type, &draft_port.port_type)
            };
            !T::compatible(out_type, in_type)
        })
    });

    let id_source = id.clone();
    let reg_source = registry.clone();
    let is_source = Signal::derive(move || {
        reg_source
            .draft_connection
            .with(|d| d.as_ref().is_some_and(|d| d.source_port == id_source))
    });

    let id_conn = id.clone();
    let reg_conn = registry.clone();
    let is_connected = Signal::derive(move || {
        reg_conn.connections.with(|conns| {
            conns
                .values()
                .any(|c| c.source == id_conn || c.target == id_conn)
        })
    });

    // Broken connections: one side registered, other side missing
    let id_broken = id.clone();
    let reg_broken = registry.clone();
    let has_broken_connections = Signal::derive(move || {
        reg_broken.connections.with(|conns| {
            reg_broken.ports.with_untracked(|ports| {
                conns.values().any(|c| {
                    let involves_me = c.source == id_broken || c.target == id_broken;
                    if !involves_me {
                        return false;
                    }
                    let source_ok = ports.contains_key(&c.source);
                    let target_ok = ports.contains_key(&c.target);
                    source_ok != target_ok // exactly one side missing
                })
            })
        })
    });

    // Context menu state
    let ctx_menu_pos: RwSignal<Option<Position>> = RwSignal::new(None);

    // A consumer-supplied builder replaces the built-in items wholesale — it can
    // name the far end of each wire, which the library can't.
    let custom_menu = use_context::<AnchorMenuBuilder<P>>();

    let id_menu = id.clone();
    let reg_menu = registry.clone();
    let menu_items = Signal::derive(move || {
        if let Some(builder) = custom_menu.clone() {
            return builder.build(&id_menu, direction);
        }
        let has_conns = reg_menu.connections.with(|conns| {
            conns
                .values()
                .any(|c| c.source == id_menu || c.target == id_menu)
        });
        let has_broken = reg_menu.connections.with(|conns| {
            reg_menu.ports.with_untracked(|ports| {
                conns.values().any(|c| {
                    let involves_me = c.source == id_menu || c.target == id_menu;
                    if !involves_me {
                        return false;
                    }
                    !ports.contains_key(&c.source) || !ports.contains_key(&c.target)
                })
            })
        });
        vec![
            AnchorMenuItem {
                label: "Remove connections".into(),
                action: AnchorMenuAction::RemoveConnections,
                enabled: has_conns,
            },
            AnchorMenuItem {
                label: "Remove broken connections".into(),
                action: AnchorMenuAction::RemoveBrokenConnections,
                enabled: has_broken,
            },
        ]
    });

    let id_action = id.clone();
    let reg_action = registry.clone();
    let on_action = Callback::new(move |action: AnchorMenuAction| {
        ctx_menu_pos.set(None);
        match action {
            AnchorMenuAction::RemoveConnections => {
                let to_remove: Vec<_> = reg_action.connections.with_untracked(|conns| {
                    conns
                        .values()
                        .filter(|c| c.source == id_action || c.target == id_action)
                        .map(|c| c.id.clone())
                        .collect()
                });
                for conn_id in to_remove {
                    reg_action.emit(GraphEvent::ConnectionRemoved { id: conn_id });
                }
            }
            AnchorMenuAction::RemoveBrokenConnections => {
                let to_remove: Vec<_> = reg_action.connections.with_untracked(|conns| {
                    reg_action.ports.with_untracked(|ports| {
                        conns
                            .values()
                            .filter(|c| {
                                let involves = c.source == id_action || c.target == id_action;
                                let broken = !ports.contains_key(&c.source)
                                    || !ports.contains_key(&c.target);
                                involves && broken
                            })
                            .map(|c| c.id.clone())
                            .collect()
                    })
                });
                for conn_id in to_remove {
                    reg_action.emit(GraphEvent::ConnectionRemoved { id: conn_id });
                }
            }
            AnchorMenuAction::Custom(on_select) => on_select.run(()),
        }
    });

    let on_close = Callback::new(move |_: ()| {
        ctx_menu_pos.set(None);
    });

    let menu_state = AnchorMenuState {
        position: ctx_menu_pos,
        items: menu_items,
        on_action,
        on_close,
    };
    provide_context(menu_state);

    // Right-click handler. An empty item list means this anchor has nothing to
    // offer — swallow the browser menu but don't open an empty panel.
    let _ = use_event_listener(
        anchor_ref,
        leptos::ev::contextmenu,
        move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            if menu_items.with_untracked(|items| items.is_empty()) {
                return;
            }
            ctx_menu_pos.set(Some(Position::new(
                ev.client_x() as f64,
                ev.client_y() as f64,
            )));
        },
    );

    // Close the context menu on a press anywhere else. NOT on a press inside the
    // menu itself: items act on pointerup, so closing here would unmount the item
    // out from under its own release and the action would never run.
    let _ = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::pointerdown,
        move |ev: web_sys::PointerEvent| {
            use leptos::wasm_bindgen::JsCast;
            let in_menu = ev
                .target()
                .and_then(|t| t.dyn_ref::<web_sys::Element>().cloned())
                .is_some_and(|el| el.closest("[data-anchor-menu]").ok().flatten().is_some());
            if !in_menu {
                ctx_menu_pos.set(None);
            }
        },
    );

    // Provide anchor context
    let anchor_ctx = AnchorContext {
        direction,
        is_compatible,
        is_incompatible,
        is_source,
        is_connected,
        has_broken_connections,
        dot_ref,
        port_type_label: port_type.type_id(),
    };
    provide_context(anchor_ctx);

    // Built-in anchor rendering: row > dot (with tooltip) > label or children
    let as_ = use_context::<crate::theme::AnchorStyle>().unwrap_or_default();
    let as2 = as_.clone();
    let as3 = as_.clone();

    let type_label = port_type.type_id();
    let tooltip_label = if show_type {
        match (direction, label.as_deref()) {
            (PortDirection::Output, Some(label)) if label != type_label => {
                format!("{label} ({type_label})")
            }
            (PortDirection::Output, Some(label)) => label.to_string(),
            _ => type_label,
        }
    } else {
        label.clone().unwrap_or(type_label)
    };
    let is_output = direction == PortDirection::Output;

    // Dot: an SVG path sized to `dot_size`, so shape is a real silhouette
    // (hexagon, triangle, …) and both flavors work — hollow-with-stroke when
    // idle, solid when connected/compatible.
    //
    // `position: relative; z-index: 1` keeps the dot above the node's resize
    // handle for themes that inset dots within its reach. Port geometry is
    // computed analytically (not from offsetParent), so positioning the dot
    // doesn't move anything.
    let dot_box_style = {
        let as_ = as_.clone();
        move || {
            let glow = if is_compatible.get() {
                format!("filter: {};", as_.dot_compatible_glow)
            } else {
                String::new()
            };
            format!(
                "width: {}px; height: {}px; position: relative; z-index: 1; \
                 flex-shrink: 0; transition: all 0.15s; cursor: crosshair; {glow}",
                as_.dot_size, as_.dot_size,
            )
        }
    };

    // Stroke/fill per state. The idle state uses the per-anchor color when
    // provided (solid), otherwise the global dot color with a hollow center.
    let dot_paint = {
        let as_ = as_.clone();
        move || {
            let (stroke, fill) = if is_compatible.get() || is_source.get() {
                (
                    as_.dot_compatible_color.clone(),
                    as_.dot_compatible_color.clone(),
                )
            } else if is_connected.get() {
                (
                    as_.dot_connected_color.clone(),
                    as_.dot_connected_color.clone(),
                )
            } else if let Some(c) = dot_color.clone() {
                (c.clone(), c)
            } else {
                (as_.dot_color.clone(), "none".to_string())
            };
            (stroke, fill)
        }
    };
    let dot_stroke = {
        let dot_paint = dot_paint.clone();
        Signal::derive(move || dot_paint().0)
    };
    let dot_fill = Signal::derive(move || dot_paint().1);
    // Border width is authored in px against `dot_size`; convert to viewBox units.
    let stroke_units = as_.dot_border_width * 24.0 / as_.dot_size.max(1.0);
    let shape_path = dot_shape.path();
    // Collection sockets stack a smaller ghost copy behind the primary shape,
    // offset toward the node interior (right for inputs, left for outputs).
    let ghost_transform = {
        let dx = if is_output { -5.0 } else { 5.0 };
        // scale(0.72) about the box center, then shift.
        format!(
            "translate({} {}) scale(0.72)",
            12.0 * 0.28 + dx,
            12.0 * 0.28
        )
    };

    // Row style
    let row_style = move || {
        let incompat = is_incompatible.get();
        let opacity = if incompat {
            as2.incompatible_opacity
        } else {
            1.0
        };
        let pointer = if incompat {
            "pointer-events: none;"
        } else {
            ""
        };
        let dir = if is_output {
            "flex-direction: row-reverse;"
        } else {
            ""
        };
        // With a draft in flight the whole row completes the connection, so it
        // matches the dot. Otherwise the row falls through to the node drag —
        // inherit the node's grab cursor rather than overriding it.
        let cursor = if is_compatible.get() {
            "cursor: crosshair;"
        } else {
            ""
        };

        format!(
            "display: flex; align-items: center; gap: {}; padding: {}; \
             height: {}px; overflow: hidden; \
             {cursor} transition: opacity 0.15s; opacity: {opacity}; {pointer} {dir}",
            as2.row_gap, as2.row_padding, as2.row_height,
        )
    };

    // Label style
    let label_style = move || {
        let color = if is_compatible.get() {
            &as3.label_compatible_color
        } else {
            &as3.label_color
        };
        format!(
            "font-size: {}; color: {color}; white-space: nowrap;",
            as3.label_font_size
        )
    };

    // Tooltip.
    //
    // PORTALLED to the body, for the same reason as the context menu below: the
    // dot lives inside a node card under the canvas `transform`, and a transformed
    // ancestor is the containing block for `position: fixed`, so an in-place
    // tooltip is displaced by the pan and scaled by the zoom.
    //
    // It is also anchored to the DOT, not to the pointer. The old code stored the
    // mouseenter client coords once, which meant pan/zoom slid the dot out from
    // under a stationary cursor while the tooltip stayed put. Measuring the dot
    // each time the viewport changes is what actually makes it stick.
    let (dot_hovered, set_dot_hovered) = signal(false);
    let tooltip_pos = RwSignal::new(Position::new(0.0, 0.0));
    let reg_tip = registry.clone();
    Effect::new(move || {
        if !dot_hovered.get() {
            return;
        }
        // Subscribe to the transform: a pan or zoom moves the dot in client space
        // without any mouse event firing. Leptos applies DOM updates before the
        // effect phase, so the rect we read here already reflects this frame.
        let _ = reg_tip.viewport.get();
        if let Some(el) = dot_ref.get() {
            let r = el.get_bounding_client_rect();
            // Anchor to the edge the tooltip grows away from, so the 12px gap is
            // measured from the dot itself at any zoom.
            let x = if is_output { r.left() } else { r.right() };
            tooltip_pos.set(Position::new(x, r.top() + r.height() / 2.0));
        }
    });
    let tooltip_style_cfg = use_context::<crate::theme::AnchorStyle>().unwrap_or_default();
    let tooltip_view = move || {
        if !dot_hovered.get() {
            return None;
        }
        let tp = tooltip_pos.get();
        // For output dots, tooltip appears to the left; for input, to the right
        let offset_x = if is_output { -12.0 } else { 12.0 };
        let transform = if is_output {
            "transform: translate(-100%, -50%);"
        } else {
            "transform: translateY(-50%);"
        };
        let style = format!(
            "position: fixed; left: {}px; top: {}px; {transform} \
             background: {}; border: {}; border-radius: 4px; \
             padding: 2px 6px; font-size: 10px; color: {}; white-space: nowrap; \
             pointer-events: none; z-index: 10000;",
            tp.x + offset_x,
            tp.y,
            tooltip_style_cfg.tooltip_background,
            tooltip_style_cfg.tooltip_border,
            tooltip_style_cfg.tooltip_color,
        );
        // `Portal` children are a `ChildrenFn`, so everything the markup uses has
        // to be cloned per call rather than moved.
        let label = tooltip_label.clone();
        Some(view! {
          <leptos::portal::Portal>
            <div style=style.clone() data-anchor-tooltip="">{label.clone()}</div>
          </leptos::portal::Portal>
        })
    };

    // Content: children override the label
    let label_or_children = if let Some(children) = children {
        children()
    } else if let Some(l) = label {
        view! { <span style=label_style>{l}</span> }.into_any()
    } else {
        ().into_any()
    };

    // Context menu.
    //
    // PORTALLED to the body, and not optional: the menu lives inside a node card,
    // which sits under the canvas `transform`. A transformed ancestor is the
    // containing block for `position: fixed` descendants, so an in-place menu is
    // offset by the pan/zoom AND clipped by the node's `overflow: hidden` — it
    // renders, in the DOM, invisibly in the wrong place. Out at the body, the
    // event's client coords mean what they say.
    let ms = use_context::<crate::theme::NodeMenuStyle>().unwrap_or_default();
    let ctx_menu_view = move || {
        let pos = ctx_menu_pos.get()?;
        let items = menu_items.get();
        let on_act = on_action;
        // Cloned out of the style struct: the markup below lives in a `ChildrenFn`,
        // so it may only borrow from this closure's environment.
        let item_color = ms.item_color.clone();

        let style = format!(
            "position: fixed; left: {}px; top: {}px; z-index: 10001; \
             background: {}; border: {}; border-radius: 6px; \
             box-shadow: {}; min-width: 180px; padding: 4px 0; overflow: hidden;",
            pos.x, pos.y, ms.background, ms.border, ms.shadow
        );

        Some(view! {
          // `Portal` children are a `ChildrenFn` — everything the markup uses has
          // to be cloned per call, not moved out of the closure.
          <leptos::portal::Portal>
            <div style=style.clone() data-anchor-menu="">
                {items.clone().into_iter().map(|item| {
                    let action = item.action.clone();
                    let enabled = item.enabled;
                    let item_style = if enabled {
                        format!("padding: 6px 12px; cursor: pointer; font-size: 12px; color: {};", item_color)
                    } else {
                        format!("padding: 6px 12px; font-size: 12px; color: {}; opacity: 0.35; pointer-events: none;", item_color)
                    };
                    view! {
                        <div
                            style=item_style
                            on:pointerup=move |ev: web_sys::PointerEvent| {
                                ev.stop_propagation();
                                if enabled {
                                    on_act.run(action.clone());
                                }
                            }
                        >
                            {item.label}
                        </div>
                    }
                }).collect_view()}
            </div>
          </leptos::portal::Portal>
        })
    };

    view! {
        <div
            node_ref=anchor_ref
            data-anchor=""
            style=row_style
        >
            <div
                style="display: inline-flex;"
                on:mouseenter=move |_| set_dot_hovered.set(true)
                on:mouseleave=move |_| set_dot_hovered.set(false)
            >
                <div style=dot_box_style node_ref=dot_ref data-anchor-dot="">
                    <svg
                        viewBox="0 0 24 24"
                        style="width: 100%; height: 100%; display: block; overflow: visible;"
                    >
                        {dot_multi
                            .then(|| {
                                view! {
                                    <path
                                        d=shape_path
                                        transform=ghost_transform
                                        fill=move || dot_fill.get()
                                        stroke=move || dot_stroke.get()
                                        stroke-width=stroke_units
                                        opacity="0.45"
                                    />
                                }
                            })}
                        <path
                            d=shape_path
                            fill=move || dot_fill.get()
                            stroke=move || dot_stroke.get()
                            stroke-width=stroke_units
                        />
                    </svg>
                </div>
            </div>
            {label_or_children}
        </div>
        {tooltip_view}
        {ctx_menu_view}
    }
}

#[component]
pub fn InputAnchor<N, P, C, T>(
    id: P,
    port_type: T,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] _marker: PhantomData<(N, C)>,
    #[prop(optional)] children: Option<Children>,
    /// Whether the compatibility type should be included in the socket tooltip.
    /// This does not affect the registered `port_type`.
    #[prop(default = true)]
    show_type: bool,
    /// Per-anchor socket dot color override (fill + border in the idle state).
    /// When `None`, uses `AnchorStyle.dot_color`.
    #[prop(optional, into)]
    dot_color: Option<String>,
    /// Per-anchor socket dot shape. Defaults to `DotShape::Circle`.
    #[prop(optional)]
    dot_shape: crate::theme::DotShape,
    /// Draw a smaller ghost copy of the shape behind the socket — for pins that
    /// carry a COLLECTION of the shape's type (arrays/lists).
    #[prop(optional)]
    dot_multi: bool,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(
        id,
        port_type,
        PortDirection::Input,
        label,
        children,
        show_type,
        dot_color,
        dot_shape,
        dot_multi,
    )
}

#[component]
pub fn OutputAnchor<N, P, C, T>(
    id: P,
    port_type: T,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] _marker: PhantomData<(N, C)>,
    #[prop(optional)] children: Option<Children>,
    /// Whether the compatibility type should be included in the socket tooltip.
    /// This does not affect the registered `port_type`.
    #[prop(default = true)]
    show_type: bool,
    /// Per-anchor socket dot color override (fill + border in the idle state).
    /// When `None`, uses `AnchorStyle.dot_color`.
    #[prop(optional, into)]
    dot_color: Option<String>,
    /// Per-anchor socket dot shape. Defaults to `DotShape::Circle`.
    #[prop(optional)]
    dot_shape: crate::theme::DotShape,
    /// Draw a smaller ghost copy of the shape behind the socket — for pins that
    /// carry a COLLECTION of the shape's type (arrays/lists).
    #[prop(optional)]
    dot_multi: bool,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(
        id,
        port_type,
        PortDirection::Output,
        label,
        children,
        show_type,
        dot_color,
        dot_shape,
        dot_multi,
    )
}
