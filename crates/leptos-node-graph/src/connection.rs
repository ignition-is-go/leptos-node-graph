use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::routing::RoutingStore;
use crate::types::*;
use crate::utils;

/// Corner rounding applied to routed polylines when drawing them.
const SUBWAY_CORNER_RADIUS: f64 = 6.0;

/// How connections are routed between ports.
///
/// Provided reactively by the consumer as an `RwSignal<RoutingMode>` in
/// context. When no context is present the renderer falls back to
/// [`RoutingMode::Orthogonal`] so existing embeds are unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoutingMode {
    /// Right-angle "subway map" wiring (default).
    #[default]
    Orthogonal,
    /// Classic bezier curves.
    Bezier,
}

impl RoutingMode {
    /// Build the SVG path `d` string for this routing mode.
    fn path(self, start: Position, end: Position) -> String {
        match self {
            RoutingMode::Bezier => utils::bezier_path(start, end),
            RoutingMode::Orthogonal => utils::orthogonal_path(start, end),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ConnectionEndpoints {
    source: Option<Position>,
    target: Option<Position>,
}

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
    // Reactive routing mode; absent context defaults to Orthogonal (subway).
    let routing_mode = use_context::<RwSignal<RoutingMode>>();
    let router = RoutingStore::new(&registry);
    {
        let mut previous = None;
        Effect::new(move |_| {
            let mode = routing_mode.map(|mode| mode.get()).unwrap_or_default();
            if previous.is_some_and(|previous| previous != mode) && mode == RoutingMode::Orthogonal
            {
                router.invalidate_all();
            }
            previous = Some(mode);
        });
    }
    let reg_for_each = registry.clone();
    let reg_for_children = registry.clone();
    let sc_for_each = style_config.clone();
    let connections_view = view! {
        <For
            each=move || reg_for_each.connections.keys()
            key=|id| id.clone()
            children=move |conn_id| {
                let reg_endpoints = reg_for_children.clone();
                let endpoints_id = conn_id.clone();
                let endpoints = Memo::new(move |_| {
                    let Some(connection) = reg_endpoints.connections.get(&endpoints_id) else { return ConnectionEndpoints::default(); };
                    ConnectionEndpoints {
                        source:reg_endpoints.ports.get(&connection.source).map(|port|port.position),
                        target:reg_endpoints.ports.get(&connection.target).map(|port|port.position),
                    }
                });
                let route_id = conn_id.clone();
                Effect::new(move |_| {
                    let _ = router.dirty.get(&route_id);
                    let mode = routing_mode.map(|mode|mode.get()).unwrap_or_default();
                    if mode == RoutingMode::Orthogonal {router.solve(&route_id);}
                });
                let route = router.routes.point(&conn_id);
                let endpoints_d = endpoints;
                let route_d = route.clone();
                let normal_d = move || {
                    let endpoints = endpoints_d.get();
                    match (endpoints.source, endpoints.target) {
                        (Some(source), Some(target)) => {
                            let mode = routing_mode.map(|mode| mode.get()).unwrap_or_default();
                            match route_d.get() {
                                Some(route) if mode == RoutingMode::Orthogonal && route.len() >= 2 => {
                                    utils::rounded_polyline_path(&route, SUBWAY_CORNER_RADIUS)
                                }
                                _ => mode.path(source, target),
                            }
                        }
                        _ => String::new(),
                    }
                };

                let endpoints_normal_style = endpoints;
                let reg_normal_style = reg_for_children.clone();
                let normal_style_id = conn_id.clone();
                let normal_sc = sc_for_each.clone();
                let normal_style = move || {
                    let selected = reg_normal_style
                        .selected_connections
                        .contains(&normal_style_id);
                    let (stroke, width) = if selected {
                        (&normal_sc.stroke_selected, normal_sc.stroke_width_selected)
                    } else {
                        (&normal_sc.stroke, normal_sc.stroke_width)
                    };
                    let display = if matches!(
                        endpoints_normal_style.get(),
                        ConnectionEndpoints { source: Some(_), target: Some(_) }
                    ) {
                        ""
                    } else {
                        "display: none;"
                    };
                    format!(
                        "{display} pointer-events: stroke; cursor: pointer; stroke: {stroke}; stroke-width: {width};"
                    )
                };

                let endpoints_data = endpoints;
                let reg_click = reg_for_children.clone();
                let click_id = conn_id.clone();

                let endpoints_stub_d = endpoints;
                let stub_d = move || dangling_geometry(endpoints_stub_d.get()).map_or_else(
                    String::new,
                    |(start, finish, _, _, _)| {
                        routing_mode
                            .map(|mode| mode.get())
                            .unwrap_or_default()
                            .path(start, finish)
                    },
                );
                let endpoints_stub_style = endpoints;
                let reg_stub_style = reg_for_children.clone();
                let stub_style_id = conn_id.clone();
                let stub_sc = sc_for_each.clone();
                let stub_style = move || {
                    let selected = reg_stub_style
                        .selected_connections
                        .contains(&stub_style_id);
                    let (stroke, width) = if selected {
                        (&stub_sc.stroke_selected, stub_sc.stroke_width_selected)
                    } else {
                        (&stub_sc.stroke, stub_sc.stroke_width)
                    };
                    let display = if dangling_geometry(endpoints_stub_style.get()).is_some() {
                        ""
                    } else {
                        "display: none;"
                    };
                    format!(
                        "{display} pointer-events: none; stroke: {stroke}; stroke-width: {width}; stroke-dasharray: 4 3; opacity: 0.5;"
                    )
                };

                let endpoints_q_x = endpoints;
                let endpoints_q_y = endpoints;
                let endpoints_anchor = endpoints;
                let endpoints_text_style = endpoints;
                let reg_text_style = reg_for_children.clone();
                let text_style_id = conn_id.clone();
                let text_sc = sc_for_each.clone();

                view! {
                    <path
                        d=normal_d
                        fill="none"
                        style=normal_style
                        data-connection=move || matches!(
                            endpoints_data.get(),
                            ConnectionEndpoints { source: Some(_), target: Some(_) }
                        ).then_some("")
                        on:mousedown=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            if ev.shift_key() {
                                reg_click.toggle_connection_selection(click_id.clone());
                            } else {
                                reg_click.select_connection(click_id.clone());
                            }
                        }
                    />
                    <path
                        d=stub_d
                        fill="none"
                        style=stub_style
                        data-connection-dangling=move || {
                            dangling_geometry(endpoints.get()).is_some().then_some("")
                        }
                    />
                    <text
                        x=move || dangling_geometry(endpoints_q_x.get()).map_or(0.0, |(_, _, x, _, _)| x)
                        y=move || dangling_geometry(endpoints_q_y.get()).map_or(0.0, |(_, _, _, y, _)| y)
                        style=move || {
                            let selected = reg_text_style
                                .selected_connections
                                .contains(&text_style_id);
                            let stroke = if selected {
                                &text_sc.stroke_selected
                            } else {
                                &text_sc.stroke
                            };
                            let display = if dangling_geometry(endpoints_text_style.get()).is_some() {
                                ""
                            } else {
                                "display: none;"
                            };
                            let anchor = dangling_geometry(endpoints_anchor.get())
                                .map_or("start", |(_, _, _, _, anchor)| anchor);
                            format!(
                                "{display} font-size: 10px; fill: {stroke}; opacity: 0.5; font-weight: 600; pointer-events: none; text-anchor: {anchor};"
                            )
                        }
                    >
                        "?"
                    </text>
                }
            }
        />
    };

    let reg_draft = registry.clone();
    let draft_sc = style_config.clone();
    let draft_view = move || {
        let mode = routing_mode.map(|mode| mode.get()).unwrap_or_default();
        let draft = reg_draft.draft_connection.get();
        let sc = draft_sc.clone();
        draft.map(|draft| {
            // When dragging from an input, swap so the curve flows left→right
            let (start, end) = if draft.origin_direction == PortDirection::Input {
                (draft.current_end, draft.source_position)
            } else {
                (draft.source_position, draft.current_end)
            };
            let path_d = mode.path(start, end);
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

fn dangling_geometry(
    endpoints: ConnectionEndpoints,
) -> Option<(Position, Position, f64, f64, &'static str)> {
    let (position, source_present) = match (endpoints.source, endpoints.target) {
        (Some(position), None) => (position, true),
        (None, Some(position)) => (position, false),
        _ => return None,
    };
    let end_x = position.x + if source_present { 30.0 } else { -30.0 };
    let end = Position::new(end_x, position.y);
    let (start, finish) = if source_present {
        (position, end)
    } else {
        (end, position)
    };
    Some((
        start,
        finish,
        end_x + if source_present { 6.0 } else { -6.0 },
        position.y + 4.0,
        if source_present { "start" } else { "end" },
    ))
}
