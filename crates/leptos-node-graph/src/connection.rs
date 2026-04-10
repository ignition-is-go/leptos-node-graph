use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::types::*;
use crate::utils;

/// Style configuration for connections. Consumer provides this to customize appearance.
#[derive(Clone, Debug)]
pub struct ConnectionStyle {
    pub stroke: String,
    pub stroke_selected: String,
    pub stroke_draft: String,
    pub stroke_width: f64,
    pub stroke_width_selected: f64,
}

impl Default for ConnectionStyle {
    fn default() -> Self {
        Self {
            stroke: "#71717a".into(),
            stroke_selected: "#ef4444".into(),
            stroke_draft: "#22d3ee".into(),
            stroke_width: 2.0,
            stroke_width_selected: 3.0,
        }
    }
}

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
    let style_config = use_context::<ConnectionStyle>().unwrap_or_default();
    let reg2 = registry.clone();
    let style2 = style_config.clone();

    let connections_view = move || {
        let reg = registry.clone();
        let conns = reg.connections.get();
        let selected = reg.selected_connections.get();
        let ports = reg.ports.get();
        let sc = style_config.clone();

        conns
            .values()
            .filter_map(|conn| {
                let source = ports.get(&conn.source).map(|p| p.position);
                let target = ports.get(&conn.target).map(|p| p.position);
                let is_selected = selected.contains(&conn.id);
                let conn_id = conn.id.clone();

                let stroke = if is_selected { &sc.stroke_selected } else { &sc.stroke };
                let width = if is_selected { sc.stroke_width_selected } else { sc.stroke_width };

                match (source, target) {
                    // Both ports present — normal connection
                    (Some(src), Some(tgt)) => {
                        let path_d = utils::bezier_path(src, tgt);
                        let style = format!(
                            "pointer-events: stroke; cursor: pointer; stroke: {}; stroke-width: {};",
                            stroke, width
                        );
                        let reg_click = reg.clone();
                        Some(view! {
                            <path
                                d=path_d
                                fill="none"
                                style=style
                                data-connection=""
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
                        }.into_any())
                    }
                    // One port missing — dangling stub with "?" indicator
                    (Some(pos), None) | (None, Some(pos)) => {
                        let is_source_present = source.is_some();
                        let stub_len = 30.0;
                        let end_x = if is_source_present { pos.x + stub_len } else { pos.x - stub_len };
                        let end = Position::new(end_x, pos.y);
                        let (start, finish) = if is_source_present { (pos, end) } else { (end, pos) };
                        let path_d = utils::bezier_path(start, finish);
                        let style = format!(
                            "pointer-events: none; stroke: {}; stroke-width: {}; stroke-dasharray: 4 3; opacity: 0.5;",
                            stroke, width
                        );
                        let q_x = end_x + if is_source_present { 6.0 } else { -6.0 };
                        let text_anchor = if is_source_present { "start" } else { "end" };
                        Some(view! {
                            <path d=path_d fill="none" style=style data-connection-dangling="" />
                            <text
                                x=q_x
                                y=pos.y + 4.0
                                style=format!(
                                    "font-size: 10px; fill: {}; opacity: 0.5; font-weight: 600; \
                                     pointer-events: none; text-anchor: {};",
                                    stroke, text_anchor
                                )
                            >
                                "?"
                            </text>
                        }.into_any())
                    }
                    // Both missing — skip
                    (None, None) => None,
                }
            })
            .collect_view()
    };

    let draft_view = move || {
        let draft = reg2.draft_connection.get();
        let sc = style2.clone();
        draft.map(|d| {
            // When dragging from an input, swap so the curve flows left→right
            let (start, end) = if d.origin_direction == PortDirection::Input {
                (d.current_end, d.source_position)
            } else {
                (d.source_position, d.current_end)
            };
            let path_d = utils::bezier_path(start, end);
            let style = format!(
                "pointer-events: none; stroke: {}; stroke-width: {}; stroke-dasharray: 6 4;",
                sc.stroke_draft, sc.stroke_width
            );
            view! {
                <path
                    d=path_d
                    fill="none"
                    style=style
                    data-connection-draft=""
                />
            }
        })
    };

    view! {
        <svg style="position: absolute; top: 0; left: 0; width: 10000px; height: 10000px; pointer-events: none; overflow: visible;">
            {connections_view}
            {draft_view}
        </svg>
    }
}
