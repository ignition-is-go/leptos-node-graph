use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::use_event_listener;

use crate::connection::ConnectionRenderer;
use crate::interaction;
use crate::menu::{NodeMenu, NodeMenuItem, NodeMenuEvent, DraftContext};
use crate::registry::{ConnectionEntry, EditorRegistry};
use crate::selection::SelectionBox;
use crate::types::*;

#[component]
pub fn NodeEditor<N, P, C, T>(
    #[prop(into)] config: EditorConfig,
    #[prop(into)] connections: Signal<HashMap<C, ConnectionEntry<P, C>>>,
    on_event: Callback<GraphEvent<N, P, C>>,
    #[prop(optional)] _marker: PhantomData<T>,
    /// Optional node catalog for the creation menu.
    /// If provided, Tab/double-click opens a searchable menu.
    /// Consumer filters this list reactively based on `menu_search`.
    #[prop(optional, into)]
    menu_items: Option<Signal<Vec<NodeMenuItem>>>,
    /// Two-way search text for the menu. Consumer reads this to filter menu_items.
    #[prop(optional)]
    menu_search: Option<RwSignal<String>>,
    children: Children,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = EditorRegistry::<N, P, C, T>::new(config, on_event.clone());
    provide_context(registry.clone());

    let container_ref = NodeRef::<leptos::html::Div>::new();

    // Menu state — owned by the editor
    let menu_open_at: RwSignal<Option<Position>> = RwSignal::new(None);
    let menu_screen_pos: RwSignal<Option<Position>> = RwSignal::new(None);
    // Track last mouse position for Tab-to-open
    let last_mouse: RwSignal<Position> = RwSignal::new(Position::new(400.0, 300.0));
    let menu_search_signal = menu_search.unwrap_or_else(|| RwSignal::new(String::new()));
    let has_menu = menu_items.is_some();

    // Auto-focus the editor container on mount
    Effect::new(move || {
        if let Some(el) = container_ref.get() {
            let _ = el.focus();
        }
    });

    // Sync external connections into registry
    let reg = registry.clone();
    Effect::new(move || {
        let conns = connections.get();
        reg.set_connections(conns);
    });

    // Event handlers
    let reg_md = registry.clone();
    let ref_md = container_ref;
    let on_mousedown = move |ev: web_sys::MouseEvent| {
        interaction::handle_canvas_mousedown(&reg_md, ev, &ref_md);
    };

    let reg_mm = registry.clone();
    let ref_mm = container_ref;
    let _mousemove_cleanup = use_event_listener(leptos::prelude::document(), leptos::ev::mousemove, move |ev: web_sys::MouseEvent| {
        last_mouse.set(Position::new(ev.client_x() as f64, ev.client_y() as f64));
        interaction::handle_canvas_mousemove(&reg_mm, ev, &ref_mm);
    });

    let reg_mu = registry.clone();
    let ref_mu = container_ref;
    let _mouseup_cleanup = use_event_listener(leptos::prelude::document(), leptos::ev::mouseup, move |ev: web_sys::MouseEvent| {
        interaction::handle_canvas_mouseup(&reg_mu, ev, &ref_mu);
    });

    let reg_wh = registry.clone();
    let ref_wh = container_ref;
    let on_wheel = move |ev: web_sys::WheelEvent| {
        interaction::handle_wheel(&reg_wh, ev, &ref_wh);
    };

    let reg_kd = registry.clone();
    let ref_kd = container_ref;
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        // Tab opens the menu
        if ev.key() == "Tab" && has_menu {
            ev.prevent_default();
            let mouse = last_mouse.get_untracked();
            let (ox, oy) = ref_kd.with_untracked(|el| {
                el.as_ref().map(|e| {
                    let r = e.get_bounding_client_rect();
                    (r.left(), r.top())
                }).unwrap_or((0.0, 0.0))
            });
            let vp = reg_kd.viewport.get_untracked();
            let canvas_pos = vp.screen_to_canvas(Position::new(mouse.x - ox, mouse.y - oy));
            menu_open_at.set(Some(canvas_pos));
            menu_screen_pos.set(Some(mouse));
            return;
        }
        interaction::handle_keydown(&reg_kd, ev, &ref_kd);
    };

    // Double-click opens the menu
    let reg_dbl = registry.clone();
    let on_dblclick = move |ev: web_sys::MouseEvent| {
        if !has_menu { return; }
        // Only on empty canvas
        if let Some(target) = ev.target() {
            use leptos::wasm_bindgen::JsCast;
            if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                if el.closest("[data-node]").ok().flatten().is_some() { return; }
                if el.closest("[data-anchor]").ok().flatten().is_some() { return; }
            }
        }
        let vp = reg_dbl.viewport.get_untracked();
        let container_rect = container_ref.with_untracked(|el| {
            el.as_ref().map(|e| e.get_bounding_client_rect())
        });
        let (ox, oy) = container_rect
            .as_ref()
            .map(|r| (r.left(), r.top()))
            .unwrap_or((0.0, 0.0));
        let canvas_pos = vp.screen_to_canvas(Position::new(
            ev.client_x() as f64 - ox,
            ev.client_y() as f64 - oy,
        ));
        menu_open_at.set(Some(canvas_pos));
        menu_screen_pos.set(Some(Position::new(ev.client_x() as f64, ev.client_y() as f64)));
    };

    // Menu event handler — emits GraphEvent::CreateNode and clears draft
    let reg_menu = registry.clone();
    let on_event_menu = on_event.clone();
    let on_menu_event = Callback::new(move |event: NodeMenuEvent| {
        match event {
            NodeMenuEvent::CreateNode { item_id, position, connect_to_port } => {
                // Capture draft info before clearing
                let (connect_from, connect_dir) = reg_menu.draft_connection.with_untracked(|d| {
                    match d {
                        Some(d) => (Some(d.source_port.clone()), Some(d.origin_direction)),
                        None => (None, None),
                    }
                });

                // Clear the draft
                reg_menu.draft_connection.set(None);

                // Emit to consumer
                on_event_menu.run(GraphEvent::CreateNode {
                    item_id,
                    position,
                    connect_from,
                    connect_to: connect_to_port,
                    connect_direction: connect_dir,
                });
            }
            NodeMenuEvent::Cancelled => {
                reg_menu.draft_connection.set(None);
            }
        }
    });

    // Draft context for the menu (drives port sub-item visibility)
    let reg_draft = registry.clone();
    let draft_context = Signal::derive(move || {
        reg_draft.draft_connection.with(|d| {
            d.as_ref().map(|d| DraftContext {
                origin_direction: d.origin_direction,
            })
        })
    });

    let reg_vp = registry.clone();
    let canvas_transform = move || {
        let vp = reg_vp.viewport.get();
        format!(
            "position: relative; transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
            vp.pan_x, vp.pan_y, vp.zoom
        )
    };

    // Menu items (use empty vec if not provided)
    let menu_items_signal = menu_items.unwrap_or_else(|| Signal::derive(|| vec![]));

    let menu_screen_pos_signal = Signal::derive(move || menu_screen_pos.get());

    view! {
        <div
            class="node-editor"
            tabindex="0"
            node_ref=container_ref
            style="position: relative; width: 100%; height: 100%; overflow: hidden; outline: none;"
            on:mousedown=on_mousedown
            on:wheel=on_wheel
            on:keydown=on_keydown
            on:dblclick=on_dblclick
        >
            <div class="node-editor__canvas" style=canvas_transform>
                <ConnectionRenderer<N, P, C, T> />
                {children()}
            </div>
            <SelectionBox<N, P, C, T> />
        </div>
        <NodeMenu
            items=menu_items_signal
            search_text=menu_search_signal
            on_event=on_menu_event
            open_at=menu_open_at
            screen_pos=menu_screen_pos_signal
            draft_context=draft_context
        />
    }
}
