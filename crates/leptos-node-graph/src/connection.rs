use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::{ConnectionEntry, EditorRegistry, NodeEntry, PortEntry};
use crate::subway::{self, SubwayConnection, SubwayOptions, SubwayRect, SubwayRoutingStats};
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

/// Route every fully-connected wire in one batch, so the router can steer
/// around nodes and keep parallel runs off each other.
///
/// Connections are fed in a stable (debug-id) order: the solver lays routes
/// down shortest-first and later ones pay to cross earlier ones, so the input
/// order has to be reproducible or the picture would shuffle between renders.
fn subway_routes<N, P, C, T>(
    conns: &HashMap<C, ConnectionEntry<P, C>>,
    ports: &HashMap<P, PortEntry<N, P, T>>,
    nodes: &HashMap<N, NodeEntry<N>>,
) -> (HashMap<C, Vec<Position>>, SubwayRoutingStats)
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let mut rects: Vec<SubwayRect> = Vec::with_capacity(nodes.len());
    let mut rect_of: HashMap<&N, usize> = HashMap::with_capacity(nodes.len());
    for (id, node) in nodes {
        rect_of.insert(id, rects.len());
        rects.push(SubwayRect {
            x: node.position.x,
            y: node.position.y,
            w: node.size.width,
            h: node.size.height,
        });
    }

    let mut ordered: Vec<(&C, &ConnectionEntry<P, C>)> = conns.iter().collect();
    ordered.sort_by_cached_key(|(id, _)| format!("{id:?}"));

    let mut ids: Vec<C> = Vec::with_capacity(ordered.len());
    let mut inputs: Vec<SubwayConnection> = Vec::with_capacity(ordered.len());
    for (id, conn) in ordered {
        let (Some(src), Some(tgt)) = (ports.get(&conn.source), ports.get(&conn.target)) else {
            continue;
        };
        ids.push((*id).clone());
        inputs.push(SubwayConnection {
            start: src.position,
            end: tgt.position,
            start_rect: rect_of.get(&src.node_id).copied(),
            end_rect: rect_of.get(&tgt.node_id).copied(),
        });
    }

    let (routes, stats) =
        subway::compute_subway_routes_with_stats(&rects, &inputs, &SubwayOptions::default());
    (ids.into_iter().zip(routes).collect(), stats)
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
    let reg2 = registry.clone();
    let style2 = style_config.clone();
    let last_subway_stats = StoredValue::new(None::<(SubwayRoutingStats, usize, usize)>);

    let connections_view = move || {
        let mode = routing_mode.map(|m| m.get()).unwrap_or_default();
        let reg = registry.clone();
        let conns = reg.connections.get();
        let selected = reg.selected_connections.get();
        let ports = reg.ports.get();
        let nodes = reg.nodes.get();
        let ports_at_node_origin = ports
            .values()
            .filter(|port| {
                nodes
                    .get(&port.node_id)
                    .is_some_and(|node| utils::distance(port.position, node.position) < 0.01)
            })
            .count();
        let sc = style_config.clone();
        // Orthogonal mode routes the whole batch at once (around nodes, lanes
        // separated); bezier stays a per-wire curve.
        let (routes, stats) = match mode {
            RoutingMode::Orthogonal => subway_routes(&conns, &ports, &nodes),
            RoutingMode::Bezier => (HashMap::new(), SubwayRoutingStats::default()),
        };
        let stats_signature = (stats, ports_at_node_origin, ports.len());
        let stats_changed =
            last_subway_stats.with_value(|previous| *previous != Some(stats_signature));
        if stats_changed {
            last_subway_stats.set_value(Some(stats_signature));
            leptos::logging::log!(
                "[subway] connections={} ports_at_node_origin={}/{} grid={}x{} ({} cells) expansions={} routed={} fallbacks={} (grid={} budget={} no_path={})",
                conns.len(),
                ports_at_node_origin,
                ports.len(),
                stats.grid_width,
                stats.grid_height,
                stats.grid_cells,
                stats.expansions,
                stats.routed,
                stats.fallbacks(),
                stats.grid_fallbacks,
                stats.budget_fallbacks,
                stats.no_path_fallbacks,
            );
        }

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
                        // Routed polyline when the batch produced one; the
                        // local elbow/curve otherwise (bezier mode, or a wire
                        // that appeared after the solve).
                        let path_d = match routes.get(&conn.id) {
                            Some(pts) if pts.len() >= 2 => {
                                utils::rounded_polyline_path(pts, SUBWAY_CORNER_RADIUS)
                            }
                            _ => mode.path(src, tgt),
                        };
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
                        let path_d = mode.path(start, finish);
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
        let mode = routing_mode.map(|m| m.get()).unwrap_or_default();
        let draft = reg2.draft_connection.get();
        let sc = style2.clone();
        draft.map(|d| {
            // When dragging from an input, swap so the curve flows left→right
            let (start, end) = if d.origin_direction == PortDirection::Input {
                (d.current_end, d.source_position)
            } else {
                (d.source_position, d.current_end)
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
