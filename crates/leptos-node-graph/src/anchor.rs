use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::use_event_listener;

use crate::node::NodeContext;
use crate::registry::EditorRegistry;
use crate::types::*;

fn anchor_view<N, P, C, T>(
    id: P,
    port_type: T,
    direction: PortDirection,
    label: Option<String>,
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

    // Update port position when node position changes
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    Effect::new(move || {
        // Track node position to trigger recomputation
        let _pos = node_ctx.position.get();

        // Compute port position from DOM
        if let Some(el) = anchor_ref.get() {
            // Walk up to find the .node-editor container
            let container_rect = el
                .closest(".node-editor")
                .ok()
                .flatten()
                .map(|c| c.get_bounding_client_rect());

            if let Some(container_rect) = container_rect {
                let port_rect = el.get_bounding_client_rect();
                let viewport = reg_pos.viewport.get_untracked();

                // Screen-space position of the port center, relative to the container
                let screen_x =
                    port_rect.left() + port_rect.width() / 2.0 - container_rect.left();
                let screen_y =
                    port_rect.top() + port_rect.height() / 2.0 - container_rect.top();

                let canvas_pos = viewport.screen_to_canvas(Position::new(screen_x, screen_y));
                reg_pos.set_port_position(&id_pos, canvas_pos);
            }
        }
    });

    // Mouse down handler — use native event listener (not Leptos delegation)
    // because Leptos delegates on:mousedown to the document root, and the Node's
    // stop_propagation() prevents the anchor handler from firing.
    let reg_md = registry.clone();
    let id_md = id.clone();
    let pt = port_type.clone();
    let _ = use_event_listener(anchor_ref, leptos::ev::mousedown, move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        ev.prevent_default();

        match direction {
            PortDirection::Output => {
                // Start draft connection
                let port_pos = reg_md.port_position(&id_md);
                if let Some(pos) = port_pos {
                    reg_md.draft_connection.set(Some(DraftConnection {
                        source_port: id_md.clone(),
                        source_position: pos,
                        port_type: pt.clone(),
                        current_end: pos,
                    }));
                }
            }
            PortDirection::Input => {
                // Complete connection if compatible
                let has_draft = reg_md.draft_connection.with_untracked(|d| d.is_some());
                if has_draft && reg_md.is_compatible_target(&id_md) {
                    let draft = reg_md.draft_connection.with_untracked(|d| d.clone());
                    if let Some(draft) = draft {
                        reg_md.emit(GraphEvent::ConnectionRequested {
                            source: draft.source_port.clone(),
                            target: id_md.clone(),
                        });
                        reg_md.draft_connection.set(None);
                    }
                }
            }
        }
    });

    // Mouseup handler for drag-to-connect: if user drags from an output and
    // releases on this input anchor, complete the connection.
    let reg_mu = registry.clone();
    let id_mu = id.clone();
    let _ = use_event_listener(anchor_ref, leptos::ev::mouseup, move |ev: web_sys::MouseEvent| {
        if direction != PortDirection::Input {
            return;
        }
        let has_draft = reg_mu.draft_connection.with_untracked(|d| d.is_some());
        if has_draft && reg_mu.is_compatible_target(&id_mu) {
            ev.stop_propagation();
            let draft = reg_mu.draft_connection.with_untracked(|d| d.clone());
            if let Some(draft) = draft {
                reg_mu.emit(GraphEvent::ConnectionRequested {
                    source: draft.source_port.clone(),
                    target: id_mu.clone(),
                });
                reg_mu.draft_connection.set(None);
            }
        }
    });

    let dir_class = match direction {
        PortDirection::Input => "anchor anchor--input",
        PortDirection::Output => "anchor anchor--output",
    };

    let id_class = id.clone();
    let reg_class = registry.clone();
    let anchor_class = move || {
        let mut cls = String::from(dir_class);

        // Check if compatible with current draft
        let compatible = reg_class.draft_connection.with(|d| {
            d.is_some()
                && direction == PortDirection::Input
                && reg_class.is_compatible_target(&id_class)
        });
        if compatible {
            cls.push_str(" anchor--compatible");
        }

        // Check if connected
        let id_ref = &id_class;
        let connected = reg_class
            .connections
            .with(|conns| conns.values().any(|c| &c.source == id_ref || &c.target == id_ref));
        if connected {
            cls.push_str(" anchor--connected");
        }

        cls
    };

    let label_view = label.map(|l| {
        view! { <span class="anchor__label">{l}</span> }
    });

    view! {
        <div
            class=anchor_class
            node_ref=anchor_ref
        >
            <div class="anchor__dot" />
            {label_view}
        </div>
    }
}

#[component]
pub fn InputAnchor<N, P, C, T>(
    id: P,
    port_type: T,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] _marker: PhantomData<(N, C)>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Input, label)
}

#[component]
pub fn OutputAnchor<N, P, C, T>(
    id: P,
    port_type: T,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] _marker: PhantomData<(N, C)>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    anchor_view::<N, P, C, T>(id, port_type, PortDirection::Output, label)
}
