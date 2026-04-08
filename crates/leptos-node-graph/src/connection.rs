use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::types::*;
use crate::utils;

#[component]
pub fn ConnectionRenderer<N, P, C, T>(
    #[prop(optional)] _marker: PhantomData<(N, P, C, T)>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let reg2 = registry.clone();
    let reg3 = registry.clone();

    // Reactive: connections, ports, selected_connections, draft_connection, viewport
    let connections_view = move || {
        let reg = registry.clone();
        let conns = reg.connections.get();
        let selected = reg.selected_connections.get();
        let ports = reg.ports.get();

        conns
            .values()
            .filter_map(|conn| {
                let source_pos = ports.get(&conn.source).map(|p| p.position)?;
                let target_pos = ports.get(&conn.target).map(|p| p.position)?;
                let path_d = utils::bezier_path(source_pos, target_pos);
                let is_selected = selected.contains(&conn.id);
                let conn_id = conn.id.clone();
                let class = if is_selected {
                    "connection connection--selected"
                } else {
                    "connection"
                };

                let reg_click = reg.clone();
                Some(view! {
                    <path
                        d=path_d
                        class=class
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        style="pointer-events: stroke; cursor: pointer;"
                        on:mousedown=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            if ev.shift_key() {
                                reg_click.selected_connections.update(|sel| {
                                    if !sel.remove(&conn_id) {
                                        sel.insert(conn_id.clone());
                                    }
                                });
                            } else {
                                reg_click.select_connection(conn_id.clone());
                            }
                        }
                    />
                })
            })
            .collect_view()
    };

    let draft_view = move || {
        let draft = reg2.draft_connection.get();
        draft.map(|d| {
            let path_d = utils::bezier_path(d.source_position, d.current_end);
            view! {
                <path
                    d=path_d
                    class="connection connection--draft"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-dasharray="6 4"
                    style="pointer-events: none;"
                />
            }
        })
    };

    let svg_style = move || {
        let vp = reg3.viewport.get();
        format!(
            "position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; overflow: visible; transform-origin: 0 0; transform: translate({}px, {}px) scale({});",
            vp.pan_x, vp.pan_y, vp.zoom
        )
    };

    view! {
        <svg style=svg_style>
            {connections_view}
            {draft_view}
        </svg>
    }
}
