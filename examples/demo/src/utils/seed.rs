use std::collections::HashMap;

use leptos::prelude::RwSignal;
use leptos_node_graph::{ConnectionEntry, GroupBox, Position};

use crate::{DynNode, utils::colors::GROUP_COLORS};

 /// Generate a demo graph with the given number of nodes, connections, and groups.
pub fn generate_demo_graph(
    num_nodes: usize,
    num_connections: usize,
) -> (
    Vec<DynNode>,
    Vec<(String, ConnectionEntry<String, String>)>,
    Vec<GroupBox<String>>,
) {
    let types = ["color_source", "mix", "math", "output"];
    let cols = 10;
    let col_spacing = 280.0;
    let row_spacing = 220.0;

    let mut nodes = Vec::new();
    for i in 0..num_nodes {
        let col = i % cols;
        let row = i / cols;
        let node_type = types[i % types.len()];
        let id = format!("{}_{}", node_type, i);
        nodes.push(DynNode {
            id,
            node_type: node_type.into(),
            position: RwSignal::new(Position::new(
                col as f64 * col_spacing + 50.0,
                row as f64 * row_spacing + 50.0,
            )),
        });
    }

    // Simple deterministic "random" connections using a linear congruential generator
    let mut seed: u64 = 42;
    let mut rng = move || -> usize {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };

    let port_map: HashMap<&str, (&[&str], &[&str])> = [
        ("color_source", (&[][..], &["color", "alpha"][..])),
        ("mix", (&["a", "b", "factor"][..], &["result"][..])),
        ("math", (&["a", "b"][..], &["result"][..])),
        ("output", (&["color", "value"][..], &[][..])),
    ]
    .into_iter()
    .collect();

    let mut connections = Vec::new();
    let mut used_targets: std::collections::HashSet<String> = std::collections::HashSet::new();

    for c in 0..num_connections * 3 {
        if connections.len() >= num_connections {
            break;
        }

        let src_idx = rng() % num_nodes;
        let dst_idx = rng() % num_nodes;
        if src_idx == dst_idx {
            continue;
        }

        let src_node = &nodes[src_idx];
        let dst_node = &nodes[dst_idx];

        let (_, src_outputs) = port_map.get(src_node.node_type.as_str()).unwrap();
        let (dst_inputs, _) = port_map.get(dst_node.node_type.as_str()).unwrap();

        if src_outputs.is_empty() || dst_inputs.is_empty() {
            continue;
        }

        let src_port = src_outputs[rng() % src_outputs.len()];
        let dst_port = dst_inputs[rng() % dst_inputs.len()];

        let source = format!("{}_{}", src_node.id, src_port);
        let target = format!("{}_{}", dst_node.id, dst_port);

        // Only one connection per input
        if used_targets.contains(&target) {
            continue;
        }
        used_targets.insert(target.clone());

        let conn_id = format!("conn_{}", c);
        connections.push((
            conn_id.clone(),
            ConnectionEntry {
                id: conn_id,
                source,
                target,
            },
        ));
    }

    // Generate groups: cluster every 2 rows into a group
    let mut groups = Vec::new();
    let rows = num_nodes.div_ceil(cols);
    let group_size = 2; // rows per group
    let mut group_idx = 0;
    let mut row = 0;
    while row < rows {
        let end_row = (row + group_size).min(rows);
        let start_idx = row * cols;
        let end_idx = (end_row * cols).min(num_nodes);
        if start_idx < end_idx {
            let node_ids: Vec<String> = nodes[start_idx..end_idx]
                .iter()
                .map(|n| n.id.clone())
                .collect();
            let color = GROUP_COLORS[group_idx % GROUP_COLORS.len()];
            groups.push(GroupBox {
                id: format!("group_{}", group_idx),
                node_ids,
                label: Some(format!("Group {}", group_idx + 1)),
                color: Some(color.into()),
                error: false,
            });
            group_idx += 1;
        }
        row = end_row;
    }

    (nodes, connections, groups)
}
