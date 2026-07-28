use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::{UseElementSizeReturn, use_debounce_fn, use_element_size, use_event_listener};

use crate::connection::ConnectionRenderer;
use crate::group::GroupBoxOverlay;
use crate::interaction;
use crate::menu::{DraftContext, NodeMenu, NodeMenuEvent, NodeMenuItem};
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
    /// Optional groups to render as visual overlays behind nodes.
    #[prop(optional, into)]
    groups: Option<Signal<Vec<crate::group::GroupBox<N>>>>,
    /// Callback for group events (rename, add/remove node).
    #[prop(optional, into)]
    on_group_event: Option<Callback<crate::group::GroupEvent<N>>>,
    children: Children,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = EditorRegistry::<N, P, C, T>::new(config, on_event);
    provide_context(registry.clone());

    let container_ref = NodeRef::<leptos::html::Div>::new();

    // Measure the container so each node can compute whether it's within the
    // visible viewport (used to cull the expensive live content of off-screen
    // nodes while keeping them mounted — their ports stay registered so wires
    // are unaffected).
    // Measured from the DOM on mount, with the observer driving later changes.
    // The observer's first callback only arrives a frame after mount (and not at
    // all while the tab is hidden and the browser throttles it), which would
    // otherwise leave framing (`F`) and viewport culling working off a 0x0
    // container until the first resize.
    let UseElementSizeReturn {
        width: cont_w,
        height: cont_h,
    } = use_element_size(container_ref);
    let reg_cs = registry.clone();
    Effect::new(move || {
        let (obs_w, obs_h) = (cont_w.get(), cont_h.get());
        if let Some(el) = container_ref.get() {
            let (w, h) = (el.offset_width() as f64, el.offset_height() as f64);
            if w > 0.0 || h > 0.0 {
                reg_cs.container_size.set(Size::new(w, h));
                return;
            }
        }
        reg_cs.container_size.set(Size::new(obs_w, obs_h));
    });

    // Debounce viewport → visibility_viewport so node visibility (and thus the
    // create/dispose of off-screen node content + its server queries) settles only
    // AFTER a pan/zoom ends. The live `viewport` still drives the CSS transform
    // every frame, so panning stays smooth; subscriptions never churn mid-pan.
    let reg_settle = registry.clone();
    let settle = use_debounce_fn(
        move || {
            let vp = reg_settle.viewport.get_untracked();
            reg_settle.visibility_viewport.set(vp);
        },
        140.0,
    );
    let reg_track = registry.clone();
    Effect::new(move || {
        let _ = reg_track.viewport.get(); // track pan/zoom
        settle();
    });

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
    let _mousemove_cleanup = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::mousemove,
        move |ev: web_sys::MouseEvent| {
            last_mouse.set(Position::new(ev.client_x() as f64, ev.client_y() as f64));
            interaction::handle_canvas_mousemove(&reg_mm, ev, &ref_mm);
        },
    );

    let reg_mu = registry.clone();
    let ref_mu = container_ref;
    let _mouseup_cleanup = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::mouseup,
        move |ev: web_sys::MouseEvent| {
            interaction::handle_canvas_mouseup(&reg_mu, ev, &ref_mu);
        },
    );

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
                el.as_ref()
                    .map(|e| {
                        let r = e.get_bounding_client_rect();
                        (r.left(), r.top())
                    })
                    .unwrap_or((0.0, 0.0))
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
        if !has_menu {
            return;
        }
        // Not while a draft connection is in flight — completing/cancelling a
        // connection can land a fast click pair on the canvas, and the draft menu
        // is reached with Tab, not by clicking. Don't pop it from a connection
        // gesture.
        if reg_dbl.draft_connection.with_untracked(Option::is_some) {
            return;
        }
        // Only on empty canvas
        if let Some(target) = ev.target() {
            use leptos::wasm_bindgen::JsCast;
            if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                if el.closest("[data-node]").ok().flatten().is_some() {
                    return;
                }
                if el.closest("[data-anchor]").ok().flatten().is_some() {
                    return;
                }
            }
        }
        let vp = reg_dbl.viewport.get_untracked();
        let container_rect =
            container_ref.with_untracked(|el| el.as_ref().map(|e| e.get_bounding_client_rect()));
        let (ox, oy) = container_rect
            .as_ref()
            .map(|r| (r.left(), r.top()))
            .unwrap_or((0.0, 0.0));
        let canvas_pos = vp.screen_to_canvas(Position::new(
            ev.client_x() as f64 - ox,
            ev.client_y() as f64 - oy,
        ));
        menu_open_at.set(Some(canvas_pos));
        menu_screen_pos.set(Some(Position::new(
            ev.client_x() as f64,
            ev.client_y() as f64,
        )));
    };

    // Menu event handler — emits GraphEvent::CreateNode and clears draft
    let reg_menu = registry.clone();
    let on_event_menu = on_event;
    let on_menu_event = Callback::new(move |event: NodeMenuEvent| {
        match event {
            NodeMenuEvent::CreateNode {
                item_id,
                position,
                connect_to_port,
            } => {
                // Capture draft info before clearing
                let (connect_from, connect_dir) =
                    reg_menu.draft_connection.with_untracked(|d| match d {
                        Some(d) => (Some(d.source_port.clone()), Some(d.origin_direction)),
                        None => (None, None),
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
        // The menu stole keyboard focus to its search input; once it closes, focus
        // lands on <body> and the editor's key handlers (Tab to reopen, delete, …)
        // go dead until the user clicks the canvas. Return focus to the editor on
        // the next frame (after the menu element is gone).
        let refocus = container_ref;
        request_animation_frame(move || {
            if let Some(el) = refocus.get_untracked() {
                let _ = el.focus();
            }
        });
    });

    // Mirror the menu's open state onto the registry so the interaction handlers
    // can see it. `menu_open_at` stays the single source of truth; this is the
    // read path for code that only has the registry (see `menu_open`).
    let reg_menu_open = registry.clone();
    Effect::new(move || {
        reg_menu_open.menu_open.set(menu_open_at.get().is_some());
    });

    // Draft context for the menu (drives port sub-item visibility + type filtering)
    let reg_draft = registry.clone();
    let compat_cb = Callback::new(|(src, tgt): (String, String)| T::compatible_by_id(&src, &tgt));
    let draft_context = Signal::derive(move || {
        reg_draft.draft_connection.with(|d| {
            d.as_ref().map(|d| {
                let source_type_id = reg_draft.ports.with_untracked(|ports| {
                    ports
                        .get(&d.source_port)
                        .map(|p| p.port_type.type_id())
                        .unwrap_or_default()
                });

                DraftContext {
                    origin_direction: d.origin_direction,
                    source_type_id,
                    is_compatible: compat_cb,
                }
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

    // Mount point for node-anchored overlays (see overlay.rs).
    let overlay_ref = NodeRef::<leptos::html::Div>::new();
    provide_context(crate::overlay::NodeOverlayLayer {
        mount: overlay_ref,
        viewport: registry.viewport.into(),
    });

    // A fast drag or resize can outrun the node and leave the pointer over empty
    // canvas — hold the gesture's cursor for its whole duration.
    let reg_cursor = registry.clone();
    let node_style_ctx = use_context::<crate::theme::NodeStyle>().unwrap_or_default();
    let (cursor_dragging, cursor_resize) =
        (node_style_ctx.cursor_dragging, node_style_ctx.cursor_resize);
    let container_style = move || {
        let base =
            "position: relative; width: 100%; height: 100%; overflow: hidden; outline: none;";
        if reg_cursor.resize_state.with(|rs| rs.is_some()) {
            format!("{base} cursor: {cursor_resize};")
        } else if reg_cursor.drag_state.with(|ds| ds.is_some()) {
            format!("{base} cursor: {cursor_dragging};")
        } else {
            base.to_string()
        }
    };

    // Menu items (use empty vec if not provided)
    let menu_items_signal = menu_items.unwrap_or_else(|| Signal::derive(std::vec::Vec::new));

    // Groups overlay
    let groups_signal = groups.unwrap_or_else(|| Signal::derive(std::vec::Vec::new));
    let groups_view = if let Some(cb) = on_group_event {
        view! {
            <GroupBoxOverlay<N, P, C, T>
                groups=groups_signal
                on_event=cb
            />
        }
        .into_any()
    } else {
        view! {
            <GroupBoxOverlay<N, P, C, T>
                groups=groups_signal
            />
        }
        .into_any()
    };

    let menu_screen_pos_signal = Signal::derive(move || menu_screen_pos.get());

    view! {
        <div
            class="node-editor"
            tabindex="0"
            node_ref=container_ref
            style=container_style
            on:mousedown=on_mousedown
            on:wheel=on_wheel
            on:keydown=on_keydown
            on:dblclick=on_dblclick
        >
            <div class="node-editor__canvas" style=canvas_transform>
                <ConnectionRenderer<N, P, C, T> />
                {groups_view}
                {children()}
            </div>
            <SelectionBox<N, P, C, T> />
            // Overlay layer: inside the pane but OUTSIDE the canvas transform,
            // so node-anchored panels are unscaled and positioned in pane space,
            // and are clipped by the graph rather than escaping to the document.
            <div
                class="node-editor__overlays"
                node_ref=overlay_ref
                style="position: absolute; inset: 0; overflow: hidden; \
                       pointer-events: none; isolation: isolate;"
            />
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
