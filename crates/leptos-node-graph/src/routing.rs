use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

use leptos::prelude::*;

use crate::keyed::KeyedMap;
use crate::registry::{ConnectionEntry, EditorRegistry};
use crate::subway::{SubwayConnection, SubwayOptions, SubwayRect, compute_subway_routes};
use crate::types::*;

const CELL: f64 = 256.0;
const PAD: f64 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}
impl Bounds {
    fn rect(rect: SubwayRect) -> Self {
        Self {
            left: rect.x,
            top: rect.y,
            right: rect.x + rect.w,
            bottom: rect.y + rect.h,
        }
    }
    fn points(points: &[Position]) -> Option<Self> {
        let first = points.first()?;
        let mut bounds = Self {
            left: first.x,
            right: first.x,
            top: first.y,
            bottom: first.y,
        };
        for point in points {
            bounds.left = bounds.left.min(point.x);
            bounds.right = bounds.right.max(point.x);
            bounds.top = bounds.top.min(point.y);
            bounds.bottom = bounds.bottom.max(point.y);
        }
        Some(bounds)
    }
    fn padded(self, pad: f64) -> Self {
        Self {
            left: self.left - pad,
            top: self.top - pad,
            right: self.right + pad,
            bottom: self.bottom + pad,
        }
    }
    fn intersects(self, other: Self) -> bool {
        self.left <= other.right
            && self.right >= other.left
            && self.top <= other.bottom
            && self.bottom >= other.top
    }
    fn cells(self) -> impl Iterator<Item = (i64, i64)> {
        let x0 = (self.left / CELL).floor() as i64;
        let x1 = (self.right / CELL).floor() as i64;
        let y0 = (self.top / CELL).floor() as i64;
        let y1 = (self.bottom / CELL).floor() as i64;
        (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
    }
}

struct Spatial<K> {
    bounds: HashMap<K, Vec<Bounds>>,
    cells: HashMap<(i64, i64), HashSet<K>>,
}
impl<K: Clone + Eq + Hash> Default for Spatial<K> {
    fn default() -> Self {
        Self {
            bounds: HashMap::new(),
            cells: HashMap::new(),
        }
    }
}
impl<K: Clone + Eq + Hash> Spatial<K> {
    fn remove(&mut self, key: &K) {
        if let Some(parts) = self.bounds.remove(key) {
            for bounds in parts {
                for cell in bounds.cells() {
                    if let Some(keys) = self.cells.get_mut(&cell) {
                        keys.remove(key);
                        if keys.is_empty() {
                            self.cells.remove(&cell);
                        }
                    }
                }
            }
        }
    }
    fn insert(&mut self, key: K, bounds: Bounds) {
        self.insert_parts(key, vec![bounds]);
    }
    fn insert_parts(&mut self, key: K, parts: Vec<Bounds>) {
        self.remove(&key);
        for bounds in &parts {
            for cell in bounds.cells() {
                self.cells.entry(cell).or_default().insert(key.clone());
            }
        }
        self.bounds.insert(key, parts);
    }
    fn query(&self, bounds: Bounds) -> HashSet<K> {
        let mut keys = HashSet::new();
        for cell in bounds.cells() {
            if let Some(candidates) = self.cells.get(&cell) {
                keys.extend(
                    candidates
                        .iter()
                        .filter(|key| {
                            self.bounds.get(*key).is_some_and(|candidate| {
                                candidate.iter().any(|part| bounds.intersects(*part))
                            })
                        })
                        .cloned(),
                );
            }
        }
        keys
    }
}

/// Work performed since the previous read. Counts the actual retained geometry reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoutingWork {
    pub prepared_nodes: usize,
    pub prepared_connections: usize,
    pub solved_connections: usize,
}

struct Geometry<N, P: PortId, C: ConnectionId> {
    nodes: HashMap<N, SubwayRect>,
    ports: HashMap<P, (N, Position)>,
    connections: HashMap<C, ConnectionEntry<P, C>>,
    incident: HashMap<P, HashSet<C>>,
    node_ports: HashMap<N, HashSet<P>>,
    obstacles: Spatial<N>,
    route_space: Spatial<C>,
    routes: HashMap<C, Vec<Position>>,
    pending: HashSet<C>,
    work: RoutingWork,
}
impl<N: NodeId, P: PortId, C: ConnectionId> Default for Geometry<N, P, C> {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            ports: HashMap::new(),
            connections: HashMap::new(),
            incident: HashMap::new(),
            node_ports: HashMap::new(),
            obstacles: Spatial::default(),
            route_space: Spatial::default(),
            routes: HashMap::new(),
            pending: HashSet::new(),
            work: RoutingWork::default(),
        }
    }
}
impl<N: NodeId, P: PortId, C: ConnectionId> Geometry<N, P, C> {
    fn incident_node(&self, id: &N) -> HashSet<C> {
        self.node_ports
            .get(id)
            .into_iter()
            .flatten()
            .flat_map(|port| self.incident.get(port).into_iter().flatten().cloned())
            .collect()
    }
    fn invalidate_node(&mut self, id: &N, new: Option<SubwayRect>) -> HashSet<C> {
        let old = self.nodes.get(id).copied();
        if old == new {
            return HashSet::new();
        }
        let mut affected = self.incident_node(id);
        for rect in old.into_iter().chain(new) {
            affected.extend(
                self.route_space
                    .query(Bounds::rect(rect).padded(PAD))
                    .into_iter()
                    .filter(|id| {
                        self.routes.get(id).is_some_and(|route| {
                            crate::subway::route_intersects_rect(route, rect, PAD)
                        })
                    }),
            );
        }
        self.obstacles.remove(id);
        match new {
            Some(rect) => {
                self.nodes.insert(id.clone(), rect);
                self.obstacles.insert(id.clone(), Bounds::rect(rect));
            }
            None => {
                self.nodes.remove(id);
            }
        }
        self.pending.extend(affected.iter().cloned());
        affected
    }
    fn invalidate_port(&mut self, id: &P, new: Option<(N, Position)>) -> HashSet<C> {
        let old = self.ports.get(id).cloned();
        if old == new {
            return HashSet::new();
        }
        if let Some((node, _)) = old
            && let Some(ports) = self.node_ports.get_mut(&node)
        {
            ports.remove(id);
            if ports.is_empty() {
                self.node_ports.remove(&node);
            }
        }
        match new {
            Some((node, position)) => {
                self.node_ports
                    .entry(node.clone())
                    .or_default()
                    .insert(id.clone());
                self.ports.insert(id.clone(), (node, position));
            }
            None => {
                self.ports.remove(id);
            }
        }
        let affected = self.incident.get(id).cloned().unwrap_or_default();
        self.pending.extend(affected.iter().cloned());
        affected
    }
    fn invalidate_connection(&mut self, id: &C, new: Option<ConnectionEntry<P, C>>) -> HashSet<C> {
        let old = self.connections.get(id).cloned();
        if old == new {
            return HashSet::new();
        }
        let mut affected = HashSet::from([id.clone()]);
        if let Some(route) = self.routes.get(id) {
            affected.extend(self.near_routes(route));
        }
        for connection in old.iter().chain(new.iter()) {
            for port in [&connection.source, &connection.target] {
                affected.extend(self.incident.get(port).into_iter().flatten().cloned());
            }
        }
        if let Some(old) = old {
            for port in [&old.source, &old.target] {
                if let Some(ids) = self.incident.get_mut(port) {
                    ids.remove(id);
                    if ids.is_empty() {
                        self.incident.remove(port);
                    }
                }
            }
        }
        match new {
            Some(connection) => {
                self.incident
                    .entry(connection.source.clone())
                    .or_default()
                    .insert(id.clone());
                self.incident
                    .entry(connection.target.clone())
                    .or_default()
                    .insert(id.clone());
                self.connections.insert(id.clone(), connection);
            }
            None => {
                self.connections.remove(id);
                self.routes.remove(id);
                self.route_space.remove(id);
            }
        }
        self.pending.extend(affected.iter().cloned());
        affected
    }
    fn corridor_nodes(&self, path: &[Position]) -> HashSet<N> {
        let mut nodes = HashSet::new();
        for segment in path.windows(2) {
            if let Some(bounds) = Bounds::points(segment) {
                nodes.extend(self.obstacles.query(bounds.padded(PAD)));
            }
        }
        nodes
    }
    fn near_routes(&self, path: &[Position]) -> HashSet<C> {
        let mut ids = HashSet::new();
        for segment in path.windows(2) {
            if let Some(bounds) = Bounds::points(segment) {
                let rect = SubwayRect {
                    x: bounds.left,
                    y: bounds.top,
                    w: bounds.right - bounds.left,
                    h: bounds.bottom - bounds.top,
                };
                ids.extend(
                    self.route_space
                        .query(bounds.padded(PAD))
                        .into_iter()
                        .filter(|id| {
                            self.routes.get(id).is_some_and(|route| {
                                crate::subway::route_intersects_rect(route, rect, PAD)
                            })
                        }),
                );
            }
        }
        ids
    }
    fn initial_path(&self, start: Position, end: Position) -> Vec<Position> {
        let middle = (start.x + end.x) / 2.0;
        vec![
            start,
            Position::new(middle, start.y),
            Position::new(middle, end.y),
            end,
        ]
    }
    fn cohort(&self, id: &C) -> HashSet<C> {
        let mut ids = HashSet::new();
        let mut queue = VecDeque::from([id.clone()]);
        while let Some(id) = queue.pop_front() {
            if !ids.insert(id.clone()) {
                continue;
            }
            if let Some(connection) = self.connections.get(&id) {
                for port in [&connection.source, &connection.target] {
                    queue.extend(
                        self.incident
                            .get(port)
                            .into_iter()
                            .flatten()
                            .filter(|id| !ids.contains(*id))
                            .cloned(),
                    );
                }
            }
            if let Some(route) = self.routes.get(&id) {
                queue.extend(
                    self.near_routes(route)
                        .into_iter()
                        .filter(|id| !ids.contains(id)),
                );
            }
        }
        ids
    }
    fn solve(&mut self, id: &C) -> Vec<(C, Option<Vec<Position>>)> {
        if !self.pending.contains(id) {
            return Vec::new();
        }
        let mut ids = self.cohort(id);
        let mut outputs = HashMap::new();
        loop {
            outputs.extend(self.solve_cohort(&ids));
            let expanded = self.cohort(id);
            if expanded.is_subset(&ids) {
                return outputs.into_iter().collect();
            }
            ids.extend(expanded);
        }
    }
    fn solve_cohort(&mut self, ids: &HashSet<C>) -> Vec<(C, Option<Vec<Position>>)> {
        for id in ids {
            self.pending.remove(id);
        }
        let mut ordered: Vec<_> = ids.iter().cloned().collect();
        ordered.sort_by_cached_key(|id| format!("{id:?}"));
        let mut outputs = Vec::new();
        let mut jobs = Vec::new();
        let mut node_ids = HashSet::new();
        for id in ordered {
            let Some(connection) = self.connections.get(&id) else {
                outputs.push((id, None));
                continue;
            };
            let (Some((start_node, start)), Some((end_node, end))) = (
                self.ports.get(&connection.source),
                self.ports.get(&connection.target),
            ) else {
                self.route_space.remove(&id);
                self.routes.remove(&id);
                outputs.push((id, None));
                continue;
            };
            self.work.prepared_connections += 1;
            let initial = self.initial_path(*start, *end);
            node_ids.extend(self.corridor_nodes(&initial));
            if let Some(route) = self.routes.get(&id) {
                node_ids.extend(self.corridor_nodes(route));
            }
            jobs.push((id, start_node.clone(), end_node.clone(), *start, *end));
        }
        if jobs.is_empty() {
            return outputs;
        }
        for (_, start, end, _, _) in &jobs {
            node_ids.insert(start.clone());
            node_ids.insert(end.clone());
        }
        let routes = loop {
            let mut ordered_nodes: Vec<_> = node_ids.iter().cloned().collect();
            ordered_nodes.sort_by_cached_key(|id| format!("{id:?}"));
            let mut rects = Vec::new();
            let mut slots = HashMap::new();
            for node in ordered_nodes {
                if let Some(rect) = self.nodes.get(&node) {
                    self.work.prepared_nodes += 1;
                    slots.insert(node, rects.len());
                    rects.push(*rect);
                }
            }
            let inputs: Vec<_> = jobs
                .iter()
                .map(|(_, start_node, end_node, start, end)| SubwayConnection {
                    start: *start,
                    end: *end,
                    start_rect: slots.get(start_node).copied(),
                    end_rect: slots.get(end_node).copied(),
                })
                .collect();
            let routes = compute_subway_routes(&rects, &inputs, &SubwayOptions::default());
            self.work.solved_connections += routes.len();
            // A detour may leave the initial corridor. Include obstacles near the
            // actual paths, retaining the previous context until it stops growing.
            let mut discovered = HashSet::new();
            for route in &routes {
                discovered.extend(self.corridor_nodes(route));
            }
            if discovered.is_subset(&node_ids) {
                break routes;
            }
            node_ids.extend(discovered);
        };
        for ((id, _, _, _, _), route) in jobs.into_iter().zip(routes) {
            self.route_space.insert_parts(
                id.clone(),
                route.windows(2).filter_map(Bounds::points).collect(),
            );
            self.routes.insert(id.clone(), route.clone());
            outputs.push((id, Some(route)));
        }
        outputs
    }
}

/// Retained routing geometry receives changed rows directly from the registry.
pub struct RoutingStore<N: NodeId, P: PortId, C: ConnectionId> {
    geometry: StoredValue<Geometry<N, P, C>>,
    pub dirty: KeyedMap<C, u64>,
    pub routes: KeyedMap<C, Vec<Position>>,
}
impl<N: NodeId, P: PortId, C: ConnectionId> Copy for RoutingStore<N, P, C> {}
impl<N: NodeId, P: PortId, C: ConnectionId> Clone for RoutingStore<N, P, C> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<N: NodeId, P: PortId, C: ConnectionId> RoutingStore<N, P, C> {
    pub fn new<T: PortType>(registry: &EditorRegistry<N, P, C, T>) -> Self {
        let store = Self {
            geometry: StoredValue::new(Geometry::default()),
            dirty: KeyedMap::new(),
            routes: KeyedMap::new(),
        };
        registry.nodes.with_untracked(|nodes| {
            for (id, node) in nodes {
                store.node(
                    id,
                    Some(SubwayRect {
                        x: node.position.x,
                        y: node.position.y,
                        w: node.size.width,
                        h: node.size.height,
                    }),
                );
            }
        });
        registry.ports.with_untracked(|ports| {
            for (id, port) in ports {
                store.port(id, Some((port.node_id.clone(), port.position)));
            }
        });
        registry.connections.with_untracked(|connections| {
            for (id, connection) in connections {
                store.connection(id, Some(connection.clone()));
            }
        });
        registry.nodes.subscribe(move |id, _, new| {
            store.node(
                id,
                new.map(|node| SubwayRect {
                    x: node.position.x,
                    y: node.position.y,
                    w: node.size.width,
                    h: node.size.height,
                }),
            )
        });
        registry.ports.subscribe(move |id, _, new| {
            store.port(id, new.map(|port| (port.node_id.clone(), port.position)))
        });
        registry
            .connections
            .subscribe(move |id, _, new| store.connection(id, new.cloned()));
        store
    }
    fn publish_dirty(&self, ids: HashSet<C>) {
        for id in ids {
            let revision = self.dirty.get_untracked(&id).unwrap_or(0) + 1;
            self.dirty.insert(id, revision);
        }
    }
    fn node(&self, id: &N, new: Option<SubwayRect>) {
        let mut ids = HashSet::new();
        self.geometry
            .update_value(|geometry| ids = geometry.invalidate_node(id, new));
        self.publish_dirty(ids);
    }
    fn port(&self, id: &P, new: Option<(N, Position)>) {
        let mut ids = HashSet::new();
        self.geometry
            .update_value(|geometry| ids = geometry.invalidate_port(id, new));
        self.publish_dirty(ids);
    }
    fn connection(&self, id: &C, new: Option<ConnectionEntry<P, C>>) {
        let removed = new.is_none();
        let mut ids = HashSet::new();
        self.geometry
            .update_value(|geometry| ids = geometry.invalidate_connection(id, new));
        if removed {
            self.routes.remove(id);
            self.dirty.remove(id);
            ids.remove(id);
            self.geometry.update_value(|geometry| {
                geometry.pending.remove(id);
            });
        }
        self.publish_dirty(ids);
    }
    pub fn solve(&self, id: &C) {
        let mut outputs = Vec::new();
        self.geometry
            .update_value(|geometry| outputs = geometry.solve(id));
        for (id, route) in outputs {
            match route {
                Some(route) => self.routes.insert(id, route),
                None => self.routes.remove(&id),
            }
        }
    }
    pub fn invalidate_all(&self) {
        let mut ids = HashSet::new();
        self.geometry.update_value(|geometry| {
            ids = geometry.connections.keys().cloned().collect();
            geometry.pending.extend(ids.iter().cloned());
        });
        self.publish_dirty(ids);
    }
    pub fn take_work(&self) -> RoutingWork {
        let mut work = RoutingWork::default();
        self.geometry
            .update_value(|geometry| work = std::mem::take(&mut geometry.work));
        work
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Any;
    impl PortType for Any {
        fn compatible(_: &Self, _: &Self) -> bool {
            true
        }
        fn type_id(&self) -> String {
            "any".into()
        }
        fn from_type_id(_: &str) -> Self {
            Self
        }
    }
    type Registry = EditorRegistry<usize, usize, usize, Any>;
    fn branch(reg: &Registry, id: usize, x: f64, y: f64) {
        let source = id * 2;
        let target = source + 1;
        reg.register_node(source, Position::new(x, y), None);
        reg.set_node_size(&source, Size::new(40.0, 40.0));
        reg.register_node(target, Position::new(x + 300.0, y), None);
        reg.set_node_size(&target, Size::new(40.0, 40.0));
        reg.register_port(
            source,
            source,
            PortDirection::Output,
            Any,
            Position::new(x + 40.0, y + 20.0),
        );
        reg.register_port(
            target,
            target,
            PortDirection::Input,
            Any,
            Position::new(x + 300.0, y + 20.0),
        );
        reg.connections
            .insert(id, ConnectionEntry { id, source, target });
    }
    fn registry() -> Registry {
        Registry::new(EditorConfig::default(), Callback::new(|_| {}))
    }
    fn flush(store: RoutingStore<usize, usize, usize>, reg: &Registry) {
        for id in reg.connections.keys_untracked() {
            store.solve(&id);
        }
    }

    #[test]
    fn changed_node_and_new_wire_prepare_constant_work_with_large_background() {
        let mut counts = Vec::new();
        for background in [10, 1000] {
            Owner::new().with(|| {
                let reg = registry();
                branch(&reg, 0, 0.0, 0.0);
                for id in 1..=background {
                    branch(&reg, id, 10000.0, id as f64 * 2000.0);
                }
                let store = RoutingStore::new(&reg);
                flush(store, &reg);
                store.take_work();
                let other = store.routes.point(&1);
                let before = other.get_untracked();
                reg.batch_set_positions(&[(0, Position::new(20.0, 0.0))]);
                store.solve(&0);
                let moved = store.take_work();
                assert_eq!(other.get_untracked(), before);
                reg.register_node(10000, Position::new(300.0, 100.0), None);
                reg.set_node_size(&10000, Size::new(40.0, 40.0));
                reg.register_port(
                    10000,
                    10000,
                    PortDirection::Input,
                    Any,
                    Position::new(300.0, 120.0),
                );
                reg.connections.insert(
                    10000,
                    ConnectionEntry {
                        id: 10000,
                        source: 0,
                        target: 10000,
                    },
                );
                store.solve(&10000);
                let inserted = store.take_work();
                assert_eq!(other.get_untracked(), before);
                assert!(moved.solved_connections > 0);
                assert_eq!(inserted.solved_connections, 2);
                counts.push((moved, inserted));
            });
        }
        assert_eq!(counts[0], counts[1]);
        assert_eq!(
            counts[0].0,
            RoutingWork {
                prepared_nodes: 2,
                prepared_connections: 1,
                solved_connections: 1
            }
        );
    }

    #[test]
    fn empty_diagonal_bounding_box_does_not_couple_interior_branches() {
        let mut counts = Vec::new();
        for background in [10, 1000] {
            Owner::new().with(|| {
                let reg = registry();
                branch(&reg, 0, 0.0, 0.0);
                reg.batch_set_positions(&[(1, Position::new(1000.0, 1000.0))]);
                for id in 1..=background {
                    branch(&reg, id, 100.0, 400.0 + id as f64 * 0.1);
                }
                let store = RoutingStore::new(&reg);
                // Frozen straight interior routes are spatially independent of the
                // long L-shaped path. Seed their geometry without solving that cohort.
                store.geometry.update_value(|geometry| {
                    for id in 1..=background {
                        let route = vec![
                            Position::new(140.0, 420.0 + id as f64 * 0.1),
                            Position::new(400.0, 420.0 + id as f64 * 0.1),
                        ];
                        geometry.route_space.insert_parts(
                            id,
                            route.windows(2).filter_map(Bounds::points).collect(),
                        );
                        geometry.routes.insert(id, route);
                        geometry.pending.remove(&id);
                    }
                    let route = vec![
                        Position::new(40.0, 20.0),
                        Position::new(988.0, 20.0),
                        Position::new(988.0, 1020.0),
                        Position::new(1000.0, 1020.0),
                    ];
                    geometry
                        .route_space
                        .insert_parts(0, route.windows(2).filter_map(Bounds::points).collect());
                    geometry.routes.insert(0, route);
                });
                store.solve(&0);
                let initial = store.take_work();
                assert_eq!(initial.solved_connections, 1);
                assert_eq!(initial.prepared_nodes, 2);
                let other_revision = store.dirty.get_untracked(&1);
                reg.batch_set_positions(&[(0, Position::new(20.0, 0.0))]);
                store.solve(&0);
                let moved = store.take_work();
                assert_eq!(moved.solved_connections, 1);
                assert_eq!(store.dirty.get_untracked(&1), other_revision);
                counts.push((initial, moved));
            });
        }
        assert_eq!(counts[0], counts[1]);
    }

    #[test]
    fn obstacle_add_move_and_remove_invalidate_near_routes_using_both_bounds() {
        Owner::new().with(|| {
            let reg = registry();
            branch(&reg, 0, 0.0, 0.0);
            branch(&reg, 1, 2000.0, 0.0);
            let store = RoutingStore::new(&reg);
            flush(store, &reg);
            store.take_work();
            let far_revision = store.dirty.get_untracked(&1);
            reg.register_node(99, Position::new(130.0, 5.0), None);
            reg.set_node_size(&99, Size::new(40.0, 40.0));
            store.solve(&0);
            let added = store.take_work();
            assert_eq!(added.solved_connections, 1);
            assert_eq!(store.dirty.get_untracked(&1), far_revision);
            reg.set_node_position(&99, Position::new(2100.0, 5.0));
            store.solve(&0);
            store.solve(&1);
            assert_eq!(store.take_work().solved_connections, 2);
            reg.deregister_node(&99);
            store.solve(&1);
            assert_eq!(store.take_work().solved_connections, 1);
        });
    }

    #[test]
    fn missing_ports_restore_and_connection_removal_repairs_only_shared_cohort() {
        Owner::new().with(|| {
            let reg = registry();
            branch(&reg, 0, 0.0, 0.0);
            branch(&reg, 1, 2000.0, 0.0);
            let store = RoutingStore::new(&reg);
            flush(store, &reg);
            store.take_work();
            let far = store.routes.get_untracked(&1);
            reg.deregister_port(&1);
            store.solve(&0);
            assert_eq!(store.routes.get_untracked(&0), None);
            assert_eq!(store.routes.get_untracked(&1), far);
            reg.register_port(1, 1, PortDirection::Input, Any, Position::new(300.0, 20.0));
            store.solve(&0);
            assert!(store.routes.get_untracked(&0).is_some());
            reg.connections.insert(
                2,
                ConnectionEntry {
                    id: 2,
                    source: 0,
                    target: 1,
                },
            );
            store.solve(&2);
            store.take_work();
            reg.connections.remove(&2);
            store.solve(&0);
            assert_eq!(store.take_work().solved_connections, 1);
            assert_eq!(store.routes.get_untracked(&2), None);
            assert_eq!(store.routes.get_untracked(&1), far);
        });
    }
}
