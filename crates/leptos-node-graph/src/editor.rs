use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;

use crate::connection::ConnectionRenderer;
use crate::interaction;
use crate::registry::{ConnectionEntry, EditorRegistry};
use crate::selection::SelectionBox;
use crate::types::*;

#[component]
pub fn NodeEditor<N, P, C, T>(
    #[prop(into)] config: EditorConfig,
    #[prop(into)] connections: Signal<HashMap<C, ConnectionEntry<P, C>>>,
    on_event: Callback<GraphEvent<N, P, C>>,
    #[prop(optional)] _marker: PhantomData<T>,
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
    let on_mousemove = move |ev: web_sys::MouseEvent| {
        interaction::handle_canvas_mousemove(&reg_mm, ev, &ref_mm);
    };

    let reg_mu = registry.clone();
    let ref_mu = container_ref;
    let on_mouseup = move |ev: web_sys::MouseEvent| {
        interaction::handle_canvas_mouseup(&reg_mu, ev, &ref_mu);
    };

    let reg_wh = registry.clone();
    let ref_wh = container_ref;
    let on_wheel = move |ev: web_sys::WheelEvent| {
        interaction::handle_wheel(&reg_wh, ev, &ref_wh);
    };

    let reg_kd = registry.clone();
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        interaction::handle_keydown(&reg_kd, ev);
    };

    let reg_vp = registry.clone();
    let canvas_transform = move || {
        let vp = reg_vp.viewport.get();
        format!(
            "transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
            vp.pan_x, vp.pan_y, vp.zoom
        )
    };

    view! {
        <div
            class="node-editor"
            tabindex="0"
            node_ref=container_ref
            style="position: relative; width: 100%; height: 100%; overflow: hidden; outline: none;"
            on:mousedown=on_mousedown
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:wheel=on_wheel
            on:keydown=on_keydown
        >
            <div class="node-editor__canvas" style=canvas_transform>
                {children()}
            </div>
            <ConnectionRenderer<N, P, C, T> />
            <SelectionBox<N, P, C, T> />
        </div>
    }
}
