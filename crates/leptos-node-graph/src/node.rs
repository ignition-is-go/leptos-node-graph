use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::{UseElementSizeReturn, use_element_size, use_event_listener};

use crate::registry::{DragState, EditorRegistry};
// AnchorStyle is read by anchor.rs, not directly by node.rs
use crate::types::*;

/// Context provided by a Node to its children.
#[derive(Clone)]
pub struct NodeContext<N: NodeId> {
    pub id: N,
    pub position: RwSignal<Position>,
    pub is_selected: Signal<bool>,
    pub is_dragging: Signal<bool>,
    /// Measured height of the header + body sections above the ports.
    /// Used by anchors for deterministic position calculation.
    pub ports_y_offset: Signal<f64>,
    /// Measured node width. Used by output anchors for X position.
    pub node_width: Signal<f64>,
}

#[component]
pub fn Node<N, P, C, T>(
    id: N,
    position: RwSignal<Position>,
    #[prop(optional)] _marker: PhantomData<(P, C, T)>,
    /// Header content (title bar). Library measures its height.
    #[prop(optional)] header: Option<Children>,
    /// Body content (dropdowns, controls, etc.) between header and ports.
    /// Library measures its height.
    #[prop(optional)] body: Option<Children>,
    /// Port anchors — library lays these out in two columns.
    children: Children,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let node_ref = NodeRef::<leptos::html::Div>::new();
    let header_ref = NodeRef::<leptos::html::Div>::new();
    let body_ref = NodeRef::<leptos::html::Div>::new();

    // Measure header + body heights
    let UseElementSizeReturn { width: _header_w, height: header_h } = use_element_size(header_ref);
    let UseElementSizeReturn { width: _body_w, height: body_h } = use_element_size(body_ref);
    let UseElementSizeReturn { width: node_w, height: node_h } = use_element_size(node_ref);

    let ports_y_offset = Signal::derive(move || header_h.get() + body_h.get());
    let node_width = Signal::derive(move || node_w.get());

    // Derived state signals
    let id_sel = id.clone();
    let reg_sel = registry.clone();
    let is_selected =
        Signal::derive(move || reg_sel.selected_nodes.with(|sel| sel.contains(&id_sel)));

    let id_drag = id.clone();
    let reg_drag = registry.clone();
    let is_dragging = Signal::derive(move || {
        reg_drag
            .drag_state
            .with(|ds| ds.as_ref().is_some_and(|d| d.node_id == id_drag))
    });

    // Provide context with measurements
    let ctx = NodeContext {
        id: id.clone(),
        position,
        is_selected,
        is_dragging,
        ports_y_offset,
        node_width,
    };
    provide_context(ctx);

    // Register node
    let initial_pos = position.get_untracked();
    registry.register_node(id.clone(), initial_pos, Some(position));

    // Deregister on cleanup
    let reg_cleanup = registry.clone();
    let id_cleanup = id.clone();
    on_cleanup(move || {
        reg_cleanup.deregister_node(&id_cleanup);
    });

    // Sync position signal -> registry (skip during drag to avoid cycles)
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    Effect::new(move || {
        let pos = position.get();
        let dragging = reg_pos.drag_state.with_untracked(|ds| ds.is_some());
        if !dragging {
            reg_pos.set_node_position(&id_pos, pos);
        }
    });

    // Track node size
    let reg_size = registry.clone();
    let size_id = id.clone();
    Effect::new(move || {
        let w = node_w.get();
        let h = node_h.get();
        if w > 0.0 || h > 0.0 {
            let size = Size::new(w, h);
            reg_size.set_node_size(&size_id, size);
            reg_size.emit(GraphEvent::NodeResized {
                id: size_id.clone(),
                size,
            });
        }
    });

    // Mouse down handler
    let reg_md = registry.clone();
    let id_md = id.clone();
    let _ = use_event_listener(
        node_ref,
        leptos::ev::mousedown,
        move |ev: web_sys::MouseEvent| {
            if ev.button() != 0 {
                return;
            }

            // Skip anchor dots and form elements
            if let Some(target) = ev.target() {
                use leptos::wasm_bindgen::JsCast;
                if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                    if el.closest("[data-anchor-dot]").ok().flatten().is_some() {
                        return;
                    }
                    let tag = el.tag_name().to_uppercase();
                    if matches!(
                        tag.as_str(),
                        "INPUT" | "SELECT" | "TEXTAREA" | "BUTTON" | "OPTION"
                    ) {
                        return;
                    }
                }
            }

            ev.stop_propagation();

            let node_id = id_md.clone();

            if ev.shift_key() {
                reg_md.toggle_node_selection(node_id);
            } else {
                let already_selected = reg_md
                    .selected_nodes
                    .with_untracked(|sel| sel.contains(&node_id));
                if !already_selected {
                    reg_md.select_node(node_id.clone());
                }

                let viewport = reg_md.viewport.get_untracked();
                let container_rect = node_ref.with_untracked(|el| {
                    el.as_ref().and_then(|el| {
                        el.closest(".node-editor")
                            .ok()
                            .flatten()
                            .map(|container| container.get_bounding_client_rect())
                    })
                });

                let (offset_x, offset_y) = container_rect
                    .map(|r| (r.left(), r.top()))
                    .unwrap_or((0.0, 0.0));

                let canvas_mouse = viewport.screen_to_canvas(Position::new(
                    ev.client_x() as f64 - offset_x,
                    ev.client_y() as f64 - offset_y,
                ));

                let start_positions: HashMap<N, Position> =
                    reg_md.selected_nodes.with_untracked(|sel| {
                        reg_md.nodes.with_untracked(|nodes| {
                            sel.iter()
                                .filter_map(|nid| {
                                    nodes.get(nid).map(|entry| (nid.clone(), entry.position))
                                })
                                .collect()
                        })
                    });

                reg_md.drag_state.set(Some(DragState {
                    node_id: node_id.clone(),
                    offset: canvas_mouse,
                    start_positions,
                    alt_key: ev.alt_key(),
                }));
            }
        },
    );

    let node_style = move || {
        let pos = position.get();
        format!("position: absolute; left: {}px; top: {}px;", pos.x, pos.y)
    };

    // Render header and body slots
    let header_view = header.map(|h| h());
    let body_view = body.map(|b| b());

    view! {
        <div
            style=node_style
            node_ref=node_ref
            data-node=""
        >
            <div node_ref=header_ref data-node-header="">
                {header_view}
            </div>
            <div node_ref=body_ref data-node-body="">
                {body_view}
            </div>
            <div data-node-ports="" style="display: flex; justify-content: space-between;">
                {children()}
            </div>
        </div>
    }
}
