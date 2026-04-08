use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;
use web_sys::ResizeObserver;

use crate::registry::{DragState, EditorRegistry};
use crate::types::*;

#[derive(Clone, Debug)]
pub struct NodeContext<N: NodeId> {
    pub id: N,
    pub position: RwSignal<Position>,
}

#[component]
pub fn Node<N, P, C, T>(
    id: N,
    position: RwSignal<Position>,
    #[prop(optional)] _marker: PhantomData<(P, C, T)>,
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

    // Provide NodeContext to children
    let ctx = NodeContext {
        id: id.clone(),
        position,
    };
    provide_context(ctx);

    // Register node
    let initial_pos = position.get_untracked();
    registry.register_node(id.clone(), initial_pos);

    // Deregister on cleanup
    let reg_cleanup = registry.clone();
    let id_cleanup = id.clone();
    on_cleanup(move || {
        reg_cleanup.deregister_node(&id_cleanup);
    });

    // Sync position signal -> registry
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    Effect::new(move || {
        let pos = position.get();
        reg_pos.set_node_position(&id_pos, pos);
    });

    // ResizeObserver to track size
    // Note: The observer is set up once when the element mounts. The closure is
    // intentionally leaked (via `forget`) because wasm_bindgen Closures are not
    // Send+Sync and cannot be stored in Leptos's on_cleanup. The observer becomes
    // inert when the DOM element is removed.
    let reg_size = registry.clone();
    let id_size = id.clone();
    Effect::new(move || {
        if let Some(el) = node_ref.get() {
            let reg_inner = reg_size.clone();
            let id_inner = id_size.clone();
            let cb = Closure::<dyn Fn(js_sys::Array)>::new(move |entries: js_sys::Array| {
                if let Some(entry) = entries.get(0).dyn_ref::<web_sys::ResizeObserverEntry>() {
                    let rect = entry.content_rect();
                    let size = Size::new(rect.width(), rect.height());
                    reg_inner.set_node_size(&id_inner, size);
                    reg_inner.emit(GraphEvent::NodeResized {
                        id: id_inner.clone(),
                        size,
                    });
                }
            });
            let observer = ResizeObserver::new(cb.as_ref().unchecked_ref()).unwrap();
            observer.observe(&el);
            // Leak the closure so it lives as long as the observer
            cb.forget();
        }
    });

    // Mouse down handler for selection and drag
    let reg_md = registry.clone();
    let id_md = id.clone();
    let on_mousedown = move |ev: web_sys::MouseEvent| {
        if ev.button() != 0 {
            return;
        }
        ev.stop_propagation();

        let node_id = id_md.clone();

        if ev.shift_key() {
            reg_md.toggle_node_selection(node_id);
        } else {
            // If not already selected, select exclusively
            let already_selected = reg_md
                .selected_nodes
                .with_untracked(|sel| sel.contains(&node_id));
            if !already_selected {
                reg_md.select_node(node_id.clone());
            }

            // Start drag
            let viewport = reg_md.viewport.get_untracked();
            let container_rect = node_ref.with_untracked(|el| {
                el.as_ref().and_then(|el| {
                    el.closest(".node-editor").ok().flatten().map(|container| {
                        container.get_bounding_client_rect()
                    })
                })
            });

            let (offset_x, offset_y) = container_rect
                .map(|r| (r.left(), r.top()))
                .unwrap_or((0.0, 0.0));

            let canvas_mouse = viewport.screen_to_canvas(Position::new(
                ev.client_x() as f64 - offset_x,
                ev.client_y() as f64 - offset_y,
            ));

            // Capture start positions of all selected nodes
            let start_positions: HashMap<N, Position> = reg_md.selected_nodes.with_untracked(
                |sel| {
                    reg_md.nodes.with_untracked(|nodes| {
                        sel.iter()
                            .filter_map(|nid| {
                                nodes.get(nid).map(|entry| (nid.clone(), entry.position))
                            })
                            .collect()
                    })
                },
            );

            reg_md.drag_state.set(Some(DragState {
                node_id: node_id.clone(),
                offset: canvas_mouse,
                start_positions,
            }));
        }
    };

    let id_style = id.clone();
    let node_style = move || {
        let pos = position.get();
        format!("position: absolute; left: {}px; top: {}px;", pos.x, pos.y)
    };

    let id_class = id.clone();
    let reg_class = registry.clone();
    let node_class = move || {
        let selected = reg_class
            .selected_nodes
            .with(|sel| sel.contains(&id_class));
        let dragging = reg_class
            .drag_state
            .with(|ds| ds.as_ref().is_some_and(|d| d.node_id == id_class));
        let mut cls = String::from("node");
        if selected {
            cls.push_str(" node--selected");
        }
        if dragging {
            cls.push_str(" node--dragging");
        }
        cls
    };

    // Suppress unused variable warnings
    let _ = id_style;

    view! {
        <div
            class=node_class
            style=node_style
            node_ref=node_ref
            on:mousedown=on_mousedown
        >
            {children()}
        </div>
    }
}
