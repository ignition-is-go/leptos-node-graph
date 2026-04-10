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
}

/// Items for the anchor context menu.
#[derive(Clone, Debug)]
pub struct AnchorMenuItem {
    pub label: String,
    pub action: AnchorMenuAction,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub enum AnchorMenuAction {
    /// Remove all connections on this port.
    RemoveConnections,
    /// Remove only broken connections (where the other port is missing).
    RemoveBrokenConnections,
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
fn try_complete_connection<N, P, C, T>(
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

fn anchor_view<N, P, C, T>(
    id: P,
    port_type: T,
    direction: PortDirection,
    label: Option<String>,
    children: Option<Children>,
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

    // Update port position when node position changes.
    // Uses cached offset when available; re-measures from DOM when invalidated.
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    Effect::new(move || {
        let node_pos = node_ctx.position.get();

        // Check if we have a cached offset
        let offset = reg_pos.ports.with_untracked(|ports| {
            ports.get(&id_pos).and_then(|p| p.offset)
        });

        if let Some(offset) = offset {
            // Use cached offset (fast path — no DOM query)
            let is_dragging = reg_pos.drag_state.with_untracked(|ds| ds.is_some());
            if is_dragging {
                return; // batch_set_positions already handled this
            }
            let canvas_pos = Position::new(node_pos.x + offset.x, node_pos.y + offset.y);
            reg_pos.set_port_position(&id_pos, canvas_pos);
            return;
        }

        // No cached offset — measure from DOM (first render or after invalidation)
        // Use RAF to ensure DOM has settled after port add/remove
        let reg = reg_pos.clone();
        let id = id_pos.clone();
        let nid = node_ctx.id.clone();
        crate::raf::request_animation_frame(move || {
            let node_pos = reg.nodes.with_untracked(|nodes| {
                nodes.get(&nid).map(|n| {
                    n.position_signal.map(|s| s.get_untracked()).unwrap_or(n.position)
                }).unwrap_or_default()
            });

            if let Some(dot_el) = dot_ref.get_untracked() {
                let container_rect = dot_el
                    .closest(".node-editor")
                    .ok()
                    .flatten()
                    .map(|c| c.get_bounding_client_rect());

                if let Some(container_rect) = container_rect {
                    let port_rect = dot_el.get_bounding_client_rect();
                    let viewport = reg.viewport.get_untracked();

                    let screen_x =
                        port_rect.left() + port_rect.width() / 2.0 - container_rect.left();
                    let screen_y =
                        port_rect.top() + port_rect.height() / 2.0 - container_rect.top();

                    let canvas_pos = viewport.screen_to_canvas(Position::new(screen_x, screen_y));
                    reg.set_port_position(&id, canvas_pos);

                    let offset = Position::new(
                        canvas_pos.x - node_pos.x,
                        canvas_pos.y - node_pos.y,
                    );
                    reg.set_port_offset(&id, offset);
                }
            }
        });
    });

    // Mousedown: start a draft from any port, or click-complete an existing draft
    let reg_md = registry.clone();
    let id_md = id.clone();
    let pt = port_type.clone();
    let _ = use_event_listener(anchor_ref, leptos::ev::mousedown, move |ev: web_sys::MouseEvent| {
        let has_draft = reg_md.draft_connection.with_untracked(|d| d.is_some());

        if has_draft {
            // Complete an existing draft — clicking anywhere on the anchor row is fine
            ev.stop_propagation();
            ev.prevent_default();
            try_complete_connection(&reg_md, &id_md, direction);
        } else {
            // Start a new draft — only from the dot element
            let on_dot = if let Some(target) = ev.target() {
                use leptos::wasm_bindgen::JsCast;
                target.dyn_ref::<web_sys::Element>()
                    .map_or(false, |el| el.closest("[data-anchor-dot]").ok().flatten().is_some())
            } else {
                false
            };

            if !on_dot { return; }

            ev.stop_propagation();
            ev.prevent_default();

            // If this is a connected input, disconnect and re-route from the original output
            if direction == PortDirection::Input {
                let existing = reg_md.connections.with_untracked(|conns| {
                    conns.iter()
                        .find(|(_, c)| c.target == id_md)
                        .map(|(conn_id, c)| (conn_id.clone(), c.source.clone()))
                });

                if let Some((conn_id, source_port_id)) = existing {
                    // Remove the connection
                    reg_md.emit(GraphEvent::ConnectionRemoved { id: conn_id });

                    // Start draft from the original output
                    if let Some(source_entry) = reg_md.get_port(&source_port_id) {
                        let source_pos = reg_md.port_position(&source_port_id).unwrap_or_default();
                        reg_md.draft_connection.set(Some(DraftConnection {
                            source_port: source_port_id,
                            source_position: source_pos,
                            port_type: source_entry.port_type.clone(),
                            current_end: source_pos,
                            origin_direction: PortDirection::Output,
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
                }));
            }
        }
    });

    // Mouseup: complete a drag-to-connect on any port
    let reg_mu = registry.clone();
    let id_mu = id.clone();
    let _ = use_event_listener(anchor_ref, leptos::ev::mouseup, move |ev: web_sys::MouseEvent| {
        let has_draft = reg_mu.draft_connection.with_untracked(|d| d.is_some());
        if has_draft {
            if try_complete_connection(&reg_mu, &id_mu, direction) {
                ev.stop_propagation();
            }
        }
    });

    // Derived state signals
    // A port is a valid drop target if it's the opposite direction from the draft origin,
    // on a different node, and type-compatible.
    let id_compat = id.clone();
    let reg_compat = registry.clone();
    let is_compatible = Signal::derive(move || {
        reg_compat.draft_connection.with(|d| {
            let Some(d) = d.as_ref() else { return false };
            if direction == d.origin_direction { return false; }
            if d.source_port == id_compat { return false; }
            let Some(this_port) = reg_compat.get_port(&id_compat) else { return false };
            let Some(draft_port) = reg_compat.get_port(&d.source_port) else { return false };
            if this_port.node_id == draft_port.node_id { return false; }
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
            if d.source_port == id_incompat { return false; } // source is not incompatible
            if direction == d.origin_direction { return true; } // same direction = can't connect
            let Some(this_port) = reg_incompat.get_port(&id_incompat) else { return true };
            let Some(draft_port) = reg_incompat.get_port(&d.source_port) else { return true };
            if this_port.node_id == draft_port.node_id { return true; }
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
        reg_source.draft_connection.with(|d| {
            d.as_ref().map_or(false, |d| d.source_port == id_source)
        })
    });

    let id_conn = id.clone();
    let reg_conn = registry.clone();
    let is_connected = Signal::derive(move || {
        reg_conn.connections.with(|conns| {
            conns.values().any(|c| c.source == id_conn || c.target == id_conn)
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
                    if !involves_me { return false; }
                    let source_ok = ports.contains_key(&c.source);
                    let target_ok = ports.contains_key(&c.target);
                    source_ok != target_ok // exactly one side missing
                })
            })
        })
    });

    // Context menu state
    let ctx_menu_pos: RwSignal<Option<Position>> = RwSignal::new(None);

    let id_menu = id.clone();
    let reg_menu = registry.clone();
    let menu_items = Signal::derive(move || {
        let has_conns = reg_menu.connections.with(|conns| {
            conns.values().any(|c| c.source == id_menu || c.target == id_menu)
        });
        let has_broken = reg_menu.connections.with(|conns| {
            reg_menu.ports.with_untracked(|ports| {
                conns.values().any(|c| {
                    let involves_me = c.source == id_menu || c.target == id_menu;
                    if !involves_me { return false; }
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
                    conns.values()
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
                        conns.values()
                            .filter(|c| {
                                let involves = c.source == id_action || c.target == id_action;
                                let broken = !ports.contains_key(&c.source) || !ports.contains_key(&c.target);
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
        }
    });

    let on_close = Callback::new(move |_: ()| {
        ctx_menu_pos.set(None);
    });

    let menu_state = AnchorMenuState {
        position: ctx_menu_pos,
        items: menu_items,
        on_action: on_action.clone(),
        on_close,
    };
    provide_context(menu_state);

    // Right-click handler
    let _ = use_event_listener(anchor_ref, leptos::ev::contextmenu, move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        ctx_menu_pos.set(Some(Position::new(ev.client_x() as f64, ev.client_y() as f64)));
    });

    // Close context menu on any click elsewhere
    let _ = use_event_listener(leptos::prelude::document(), leptos::ev::pointerdown, move |_ev: web_sys::PointerEvent| {
        ctx_menu_pos.set(None);
    });

    // Provide anchor context
    let anchor_ctx = AnchorContext {
        direction,
        is_compatible,
        is_incompatible,
        is_source,
        is_connected,
        has_broken_connections,
        dot_ref,
    };
    provide_context(anchor_ctx);

    let slot_content = if let Some(children) = children {
        children()
    } else if let Some(l) = label {
        view! { <span>{l}</span> }.into_any()
    } else {
        ().into_any()
    };

    // Built-in context menu rendering
    let ms = use_context::<crate::theme::NodeMenuStyle>().unwrap_or_default();
    let ctx_menu_view = move || {
        let pos = ctx_menu_pos.get()?;
        let items = menu_items.get();
        let on_act = on_action.clone();

        let style = format!(
            "position: fixed; left: {}px; top: {}px; z-index: 10001; \
             background: {}; border: {}; border-radius: 6px; \
             box-shadow: {}; min-width: 180px; padding: 4px 0; overflow: hidden;",
            pos.x, pos.y, ms.background, ms.border, ms.shadow
        );

        Some(view! {
            <div style=style data-anchor-menu="">
                {items.into_iter().map(|item| {
                    let action = item.action.clone();
                    let on_act = on_act.clone();
                    let enabled = item.enabled;
                    let item_style = if enabled {
                        format!("padding: 6px 12px; cursor: pointer; font-size: 12px; color: {};", ms.item_color)
                    } else {
                        format!("padding: 6px 12px; font-size: 12px; color: {}; opacity: 0.35; pointer-events: none;", ms.item_color)
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
        })
    };

    view! {
        <div
            node_ref=anchor_ref
            data-anchor=""
        >
            {slot_content}
        </div>
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
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Input, label, children)
}

#[component]
pub fn OutputAnchor<N, P, C, T>(
    id: P,
    port_type: T,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] _marker: PhantomData<(N, C)>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Output, label, children)
}
