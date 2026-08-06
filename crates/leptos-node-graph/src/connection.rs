use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::{ConnectionEntry, EditorRegistry, NodeEntry, PortEntry};
use crate::subway::{self, SubwayConnection, SubwayOptions, SubwayRect, SubwayRoutingStats};
use crate::types::*;
use crate::utils;

/// Corner rounding applied to routed polylines when drawing them.
const SUBWAY_CORNER_RADIUS: f64 = 6.0;
/// Frozen routes within this distance of a changed node are included in its incremental solve.
const SUBWAY_INCREMENTAL_PROXIMITY_PAD: f64 = 48.0;

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

/// The deterministic, pure input to one subway solve.
///
/// Keeping this value separately from the result lets reactive notifications
/// which did not change world geometry skip the router entirely.
#[derive(Clone, Debug, PartialEq)]
struct SubwayBatch<N, C> {
    rect_ids: Vec<N>,
    rects: Vec<SubwayRect>,
    connections: Vec<SubwayBatchConnection<N, C>>,
}

#[derive(Clone, Debug, PartialEq)]
struct SubwayBatchConnection<N, C> {
    id: C,
    start_node: N,
    end_node: N,
    input: SubwayConnection,
}

impl<N, C> SubwayBatch<N, C>
where
    N: NodeId,
    C: ConnectionId,
{
    fn subset(&self, ids: &HashSet<C>) -> Self {
        Self {
            rect_ids: self.rect_ids.clone(),
            rects: self.rects.clone(),
            connections: self
                .connections
                .iter()
                .filter(|connection| ids.contains(&connection.id))
                .cloned()
                .collect(),
        }
    }

    /// Return the nodes responsible for a geometry-only change. Node/connection
    /// population and endpoint ownership changes are structural and require a
    /// full solve; positions and rects can be handled incrementally.
    fn changed_nodes_since(&self, previous: &Self) -> Option<HashSet<N>> {
        if self.rect_ids != previous.rect_ids
            || self.connections.len() != previous.connections.len()
        {
            return None;
        }

        let mut changed = HashSet::new();
        for ((id, current), old) in self.rect_ids.iter().zip(&self.rects).zip(&previous.rects) {
            if current != old {
                changed.insert(id.clone());
            }
        }

        for (current, old) in self.connections.iter().zip(&previous.connections) {
            if current.id != old.id
                || current.start_node != old.start_node
                || current.end_node != old.end_node
                || current.input.start_rect != old.input.start_rect
                || current.input.end_rect != old.input.end_rect
            {
                return None;
            }
            if current.input.start != old.input.start {
                changed.insert(current.start_node.clone());
            }
            if current.input.end != old.input.end {
                changed.insert(current.end_node.clone());
            }
        }

        Some(changed)
    }
}

/// Collect routing geometry in stable debug-id order. Stable node order is as
/// important as stable connection order because endpoint rects are indices.
fn subway_batch<N, P, C, T>(
    conns: &HashMap<C, ConnectionEntry<P, C>>,
    ports: &HashMap<P, PortEntry<N, P, T>>,
    nodes: &HashMap<N, NodeEntry<N>>,
) -> SubwayBatch<N, C>
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let mut ordered_nodes: Vec<(&N, &NodeEntry<N>)> = nodes.iter().collect();
    ordered_nodes.sort_by_cached_key(|(id, _)| format!("{id:?}"));

    let mut rect_ids = Vec::with_capacity(ordered_nodes.len());
    let mut rects = Vec::with_capacity(ordered_nodes.len());
    let mut rect_of: HashMap<&N, usize> = HashMap::with_capacity(ordered_nodes.len());
    for (id, node) in ordered_nodes {
        rect_of.insert(id, rects.len());
        rect_ids.push(id.clone());
        rects.push(SubwayRect {
            x: node.position.x,
            y: node.position.y,
            w: node.size.width,
            h: node.size.height,
        });
    }

    let mut ordered: Vec<(&C, &ConnectionEntry<P, C>)> = conns.iter().collect();
    ordered.sort_by_cached_key(|(id, _)| format!("{id:?}"));

    let mut connections = Vec::with_capacity(ordered.len());
    for (id, conn) in ordered {
        let (Some(src), Some(tgt)) = (ports.get(&conn.source), ports.get(&conn.target)) else {
            continue;
        };
        connections.push(SubwayBatchConnection {
            id: id.clone(),
            start_node: src.node_id.clone(),
            end_node: tgt.node_id.clone(),
            input: SubwayConnection {
                start: src.position,
                end: tgt.position,
                start_rect: rect_of.get(&src.node_id).copied(),
                end_rect: rect_of.get(&tgt.node_id).copied(),
            },
        });
    }

    SubwayBatch {
        rect_ids,
        rects,
        connections,
    }
}

fn solve_subway_batch<N, C>(
    batch: &SubwayBatch<N, C>,
) -> (HashMap<C, Vec<Position>>, SubwayRoutingStats)
where
    N: NodeId,
    C: ConnectionId,
{
    let inputs: Vec<_> = batch
        .connections
        .iter()
        .map(|connection| connection.input)
        .collect();
    let (routes, stats) =
        subway::compute_subway_routes_with_stats(&batch.rects, &inputs, &SubwayOptions::default());
    let routes = batch
        .connections
        .iter()
        .map(|connection| connection.id.clone())
        .zip(routes)
        .collect();
    (routes, stats)
}

/// Choose the small batch needed for a node geometry change: every incident
/// wire plus any frozen route close enough that the moving obstacle affects it.
fn incremental_route_ids<N, C>(
    batch: &SubwayBatch<N, C>,
    previous: &SubwayBatch<N, C>,
    changed_nodes: &HashSet<N>,
    cached_routes: &HashMap<C, Vec<Position>>,
) -> HashSet<C>
where
    N: NodeId,
    C: ConnectionId,
{
    // A frozen route may be close only to where an obstacle used to be. Test
    // both sides of the move so large jumps also straighten old detours.
    let affected_rects: Vec<_> = batch
        .rect_ids
        .iter()
        .zip(&batch.rects)
        .chain(previous.rect_ids.iter().zip(&previous.rects))
        .filter_map(|(id, rect)| changed_nodes.contains(id).then_some(*rect))
        .collect();

    batch
        .connections
        .iter()
        .filter(|connection| {
            changed_nodes.contains(&connection.start_node)
                || changed_nodes.contains(&connection.end_node)
                || cached_routes.get(&connection.id).is_some_and(|route| {
                    affected_rects.iter().any(|rect| {
                        subway::route_intersects_rect(
                            route,
                            *rect,
                            SUBWAY_INCREMENTAL_PROXIMITY_PAD,
                        )
                    })
                })
        })
        .map(|connection| connection.id.clone())
        .collect()
}

#[derive(Debug, PartialEq)]
enum SubwaySolve<C: ConnectionId> {
    Cached,
    Full,
    Partial(HashSet<C>),
}

fn plan_subway_solve<N, C>(
    batch: &SubwayBatch<N, C>,
    previous: Option<&SubwayBatch<N, C>>,
    cached_routes: &HashMap<C, Vec<Position>>,
) -> SubwaySolve<C>
where
    N: NodeId,
    C: ConnectionId,
{
    let Some(previous) = previous else {
        return SubwaySolve::Full;
    };
    if batch == previous {
        return SubwaySolve::Cached;
    }
    match batch.changed_nodes_since(previous) {
        Some(changed_nodes) => SubwaySolve::Partial(incremental_route_ids(
            batch,
            previous,
            &changed_nodes,
            cached_routes,
        )),
        None => SubwaySolve::Full,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ConnectionEndpoints {
    source: Option<Position>,
    target: Option<Position>,
}

#[derive(Clone)]
struct ConnectionSignals {
    endpoints: ArcRwSignal<ConnectionEndpoints>,
    route: ArcRwSignal<Option<Vec<Position>>>,
}

struct SubwayCache<N, C> {
    geometry: Option<SubwayBatch<N, C>>,
    routes: HashMap<C, Vec<Position>>,
}

impl<N, C> Default for SubwayCache<N, C> {
    fn default() -> Self {
        Self {
            geometry: None,
            routes: HashMap::new(),
        }
    }
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
    let connection_signals = StoredValue::new(HashMap::<C, ConnectionSignals>::new());
    let subway_cache = StoredValue::new(SubwayCache::<N, C>::default());
    let last_subway_stats = StoredValue::new(None::<(SubwayRoutingStats, usize, usize)>);

    // Collapse the coarse registry maps into per-connection signals. A port map
    // update still costs one cheap walk, but only paths whose endpoints really
    // changed receive a reactive notification.
    {
        let reg = registry.clone();
        Effect::new(move |_| {
            let conns = reg.connections.get();
            let ports = reg.ports.get();
            connection_signals.update_value(|signals| {
                signals.retain(|id, _| conns.contains_key(id));
                for (id, connection) in &conns {
                    let endpoints = ConnectionEndpoints {
                        source: ports.get(&connection.source).map(|port| port.position),
                        target: ports.get(&connection.target).map(|port| port.position),
                    };
                    let signals = signals
                        .entry(id.clone())
                        .or_insert_with(|| ConnectionSignals {
                            endpoints: ArcRwSignal::new(endpoints),
                            route: ArcRwSignal::new(
                                subway_cache.with_value(|cache| cache.routes.get(id).cloned()),
                            ),
                        });
                    if signals.endpoints.get_untracked() != endpoints {
                        signals.endpoints.set(endpoints);
                    }
                }
            });
        });
    }

    // Solve routes separately from rendering. The cache filters identical
    // geometry, and node-only changes merge a small partial solve into frozen routes.
    {
        let reg = registry.clone();
        Effect::new(move |_| {
            let mode = routing_mode.map(|mode| mode.get()).unwrap_or_default();
            let conns = reg.connections.get();
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

            if mode == RoutingMode::Bezier {
                subway_cache.update_value(|cache| *cache = SubwayCache::default());
                connection_signals.with_value(|signals| {
                    for signals in signals.values() {
                        if signals.route.get_untracked().is_some() {
                            signals.route.set(None);
                        }
                    }
                });
                log_subway_stats(
                    &last_subway_stats,
                    SubwayRoutingStats::default(),
                    conns.len(),
                    ports_at_node_origin,
                    ports.len(),
                );
                return;
            }

            let batch = subway_batch(&conns, &ports, &nodes);
            let solve = subway_cache.with_value(|cache| {
                plan_subway_solve(&batch, cache.geometry.as_ref(), &cache.routes)
            });

            let stats = match solve {
                SubwaySolve::Cached => return,
                SubwaySolve::Full => {
                    let (routes, stats) = solve_subway_batch(&batch);
                    subway_cache.update_value(|cache| {
                        cache.geometry = Some(batch.clone());
                        cache.routes = routes.clone();
                    });
                    connection_signals.with_value(|signals| {
                        for (id, signals) in signals {
                            let route = routes.get(id).cloned();
                            if signals.route.get_untracked() != route {
                                signals.route.set(route);
                            }
                        }
                    });
                    stats
                }
                SubwaySolve::Partial(ids) => {
                    if ids.is_empty() {
                        subway_cache.update_value(|cache| {
                            cache.geometry = Some(batch);
                        });
                        return;
                    }
                    let partial_batch = batch.subset(&ids);
                    let (routes, stats) = solve_subway_batch(&partial_batch);
                    subway_cache.update_value(|cache| {
                        cache.geometry = Some(batch);
                        cache.routes.extend(routes.clone());
                    });
                    connection_signals.with_value(|signals| {
                        for (id, route) in routes {
                            if let Some(signals) = signals.get(&id)
                                && signals.route.get_untracked().as_ref() != Some(&route)
                            {
                                signals.route.set(Some(route));
                            }
                        }
                    });
                    stats
                }
            };

            log_subway_stats(
                &last_subway_stats,
                stats,
                conns.len(),
                ports_at_node_origin,
                ports.len(),
            );
        });
    }

    let reg_for_each = registry.clone();
    let reg_for_children = registry.clone();
    let sc_for_each = style_config.clone();
    let connections_view = view! {
        <For
            each=move || {
                let conns = reg_for_each.connections.get();
                let ports = reg_for_each.ports.get_untracked();
                let mut ids: Vec<_> = conns.keys().cloned().collect();
                ids.sort_by_cached_key(|id| format!("{id:?}"));
                let mut items = Vec::with_capacity(ids.len());
                connection_signals.update_value(|signals| {
                    for id in ids {
                        let endpoints = conns.get(&id).map_or_else(
                            ConnectionEndpoints::default,
                            |connection| ConnectionEndpoints {
                                source: ports.get(&connection.source).map(|port| port.position),
                                target: ports.get(&connection.target).map(|port| port.position),
                            },
                        );
                        let signals = signals
                            .entry(id.clone())
                            .or_insert_with(|| ConnectionSignals {
                                endpoints: ArcRwSignal::new(endpoints),
                                route: ArcRwSignal::new(
                                    subway_cache.with_value(|cache| cache.routes.get(&id).cloned()),
                                ),
                            })
                            .clone();
                        items.push((id, signals));
                    }
                });
                items
            }
            key=|(id, _)| id.clone()
            children=move |(conn_id, signals)| {
                let endpoints_d = signals.endpoints.clone();
                let route_d = signals.route.clone();
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

                let endpoints_normal_style = signals.endpoints.clone();
                let reg_normal_style = reg_for_children.clone();
                let normal_style_id = conn_id.clone();
                let normal_sc = sc_for_each.clone();
                let normal_style = move || {
                    let selected = reg_normal_style
                        .selected_connections
                        .with(|selected| selected.contains(&normal_style_id));
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

                let endpoints_data = signals.endpoints.clone();
                let reg_click = reg_for_children.clone();
                let click_id = conn_id.clone();

                let endpoints_stub_d = signals.endpoints.clone();
                let stub_d = move || dangling_geometry(endpoints_stub_d.get()).map_or_else(
                    String::new,
                    |(start, finish, _, _, _)| {
                        routing_mode
                            .map(|mode| mode.get())
                            .unwrap_or_default()
                            .path(start, finish)
                    },
                );
                let endpoints_stub_style = signals.endpoints.clone();
                let reg_stub_style = reg_for_children.clone();
                let stub_style_id = conn_id.clone();
                let stub_sc = sc_for_each.clone();
                let stub_style = move || {
                    let selected = reg_stub_style
                        .selected_connections
                        .with(|selected| selected.contains(&stub_style_id));
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

                let endpoints_q_x = signals.endpoints.clone();
                let endpoints_q_y = signals.endpoints.clone();
                let endpoints_anchor = signals.endpoints.clone();
                let endpoints_text_style = signals.endpoints.clone();
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
                                reg_click.selected_connections.update(|selected| {
                                    if !selected.remove(&click_id) {
                                        selected.insert(click_id.clone());
                                    }
                                });
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
                            dangling_geometry(signals.endpoints.get()).is_some().then_some("")
                        }
                    />
                    <text
                        x=move || dangling_geometry(endpoints_q_x.get()).map_or(0.0, |(_, _, x, _, _)| x)
                        y=move || dangling_geometry(endpoints_q_y.get()).map_or(0.0, |(_, _, _, y, _)| y)
                        style=move || {
                            let selected = reg_text_style
                                .selected_connections
                                .with(|selected| selected.contains(&text_style_id));
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

fn log_subway_stats(
    last_subway_stats: &StoredValue<Option<(SubwayRoutingStats, usize, usize)>>,
    stats: SubwayRoutingStats,
    connection_count: usize,
    ports_at_node_origin: usize,
    port_count: usize,
) {
    let signature = (stats, ports_at_node_origin, port_count);
    let changed = last_subway_stats.with_value(|previous| *previous != Some(signature));
    if changed {
        last_subway_stats.set_value(Some(signature));
        leptos::logging::log!(
            "[subway] connections={} ports_at_node_origin={}/{} grid={}x{} ({} cells) expansions={} routed={} fallbacks={} (grid={} budget={} no_path={})",
            connection_count,
            ports_at_node_origin,
            port_count,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch() -> SubwayBatch<String, String> {
        SubwayBatch {
            rect_ids: vec!["moving".into(), "fixed".into(), "other".into()],
            rects: vec![
                SubwayRect {
                    x: 0.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                },
                SubwayRect {
                    x: 100.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                },
                SubwayRect {
                    x: 200.0,
                    y: 0.0,
                    w: 20.0,
                    h: 20.0,
                },
            ],
            connections: vec![
                SubwayBatchConnection {
                    id: "incident".into(),
                    start_node: "moving".into(),
                    end_node: "fixed".into(),
                    input: SubwayConnection {
                        start: Position::new(20.0, 10.0),
                        end: Position::new(100.0, 10.0),
                        start_rect: Some(0),
                        end_rect: Some(1),
                    },
                },
                SubwayBatchConnection {
                    id: "nearby".into(),
                    start_node: "fixed".into(),
                    end_node: "other".into(),
                    input: SubwayConnection {
                        start: Position::new(120.0, 10.0),
                        end: Position::new(200.0, 10.0),
                        start_rect: Some(1),
                        end_rect: Some(2),
                    },
                },
            ],
        }
    }

    fn cached_routes() -> HashMap<String, Vec<Position>> {
        HashMap::from([
            (
                "incident".to_string(),
                vec![Position::new(20.0, 10.0), Position::new(100.0, 10.0)],
            ),
            (
                "nearby".to_string(),
                vec![Position::new(-50.0, 30.0), Position::new(50.0, 30.0)],
            ),
        ])
    }

    #[test]
    fn programmatic_single_node_move_plans_incident_and_nearby_routes() {
        let previous = batch();
        let mut current = previous.clone();
        current.rects[0].x += 5.0;
        current.connections[0].input.start.x += 5.0;

        assert_eq!(
            plan_subway_solve(&current, Some(&previous), &cached_routes()),
            SubwaySolve::Partial(HashSet::from([
                "incident".to_string(),
                "nearby".to_string(),
            ]))
        );
    }

    #[test]
    fn incremental_plan_includes_routes_near_only_the_previous_rect() {
        let previous = batch();
        let mut current = previous.clone();
        current.rects[0].x += 500.0;
        current.connections[0].input.start.x += 500.0;
        let routes = cached_routes();
        let nearby = routes.get("nearby").unwrap();

        assert!(subway::route_intersects_rect(
            nearby,
            previous.rects[0],
            SUBWAY_INCREMENTAL_PROXIMITY_PAD,
        ));
        assert!(!subway::route_intersects_rect(
            nearby,
            current.rects[0],
            SUBWAY_INCREMENTAL_PROXIMITY_PAD,
        ));
        assert_eq!(
            plan_subway_solve(&current, Some(&previous), &routes),
            SubwaySolve::Partial(HashSet::from([
                "incident".to_string(),
                "nearby".to_string(),
            ]))
        );
    }

    #[test]
    fn endpoint_diffs_are_mapped_back_to_their_nodes() {
        let previous = batch();
        let mut current = previous.clone();
        current.connections[1].input.end.y += 2.0;

        assert_eq!(
            current.changed_nodes_since(&previous),
            Some(HashSet::from(["other".to_string()]))
        );
    }

    #[test]
    fn final_drop_with_identical_geometry_is_a_cache_hit() {
        let geometry = batch();
        assert_eq!(
            plan_subway_solve(&geometry, Some(&geometry), &cached_routes()),
            SubwaySolve::Cached
        );
    }

    #[test]
    fn structural_changes_still_request_a_full_solve() {
        let previous = batch();
        let mut current = previous.clone();
        current.connections.pop();

        assert_eq!(
            plan_subway_solve(&current, Some(&previous), &cached_routes()),
            SubwaySolve::Full
        );
    }
}
