use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_node_graph::*;
use web_sys::console;

mod nodes;
mod utils;
mod widgets;
use crate::nodes::build_node_registry;
use crate::utils::{colors::random_group_color, ids::next_id, seed::generate_demo_graph};
// ---------------------------------------------------------------------------
// Port type
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum DemoPort {
    Float,
    Color,
    Any,
}

impl PortType for DemoPort {
    fn compatible(source: &Self, target: &Self) -> bool {
        matches!(target, DemoPort::Any) || source == target
    }

    fn type_id(&self) -> String {
        format!("{self:?}")
    }

    fn from_type_id(id: &str) -> Self {
        match id {
            "Float" => DemoPort::Float,
            "Color" => DemoPort::Color,
            _ => DemoPort::Any,
        }
    }
}

// Node components, helpers, and type registry are in nodes.rs

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
    mount_to_body(App);
}

// ---------------------------------------------------------------------------
// Dynamic node storage
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DynNode {
    id: String,
    node_type: String,
    position: RwSignal<Position>,
    #[allow(dead_code)]
    category: Option<Category>,
}

/// The full node type catalog with port definitions.

#[component]
fn App() -> impl IntoView {
    let connections: RwSignal<HashMap<String, ConnectionEntry<String, String>>> =
        RwSignal::new(HashMap::new());
    let connections_signal = Signal::derive(move || connections.get());

    // Generate initial graph
    let (initial_nodes, initial_connections, initial_groups) = generate_demo_graph(2, 1);
    let nodes: RwSignal<Vec<DynNode>> = RwSignal::new(initial_nodes);
    for (id, entry) in initial_connections {
        connections.update(|map| {
            map.insert(id, entry);
        });
    }

    // Build node type registry
    let node_registry = build_node_registry();

    // Filtered catalog based on search
    let menu_search: RwSignal<String> = RwSignal::new(String::new());
    let catalog = node_registry.menu_items();
    let menu_items = Signal::derive(move || {
        let search = menu_search.get().to_lowercase();
        let items = catalog.clone();
        if search.is_empty() {
            items
        } else {
            items
                .into_iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&search)
                        || item
                            .category
                            .as_ref()
                            .is_some_and(|c| c.name.to_lowercase().contains(&search))
                        || item
                            .description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&search))
                })
                .collect()
        }
    });

    let groups: RwSignal<Vec<GroupBox<String>>> = RwSignal::new(initial_groups);

    // The app's reactive owner, captured so nodes created from a graph event get
    // signals that outlive whatever transient scope raised the event.
    //
    // Without this, `RwSignal::new` inside the handler inherits the CALLER's
    // owner — the node menu's view — which is disposed the moment the menu
    // closes. The new node then renders against a dead signal and `Node`'s
    // `position.get_untracked()` panics on a disposed signal.
    let app_owner = Owner::current().expect("app component has an owner");

    // Handed to the editor below; lets the wrapper's drop handler convert the
    // drop's client coords into canvas coords under any pan/zoom.
    let handle = EditorHandle::new();
    let drop_registry = node_registry.clone();
    let drop_owner = app_owner.clone();

    let on_event = {
        let node_registry = node_registry.clone();
        Callback::new(move |event: GraphEvent<String, String, String>| {
            console::log_1(&format!("Graph event: {:?}", event).into());
            match event {
                GraphEvent::ConnectionRequested { source, target } => {
                    let already =
                        connections.with_untracked(|c| c.values().any(|e| e.target == target));
                    if already {
                        return;
                    }
                    connections.update(|map| {
                        let id = next_id("conn");
                        map.insert(id.clone(), ConnectionEntry { id, source, target });
                    });
                }
                GraphEvent::ConnectionRemoved { id } => {
                    connections.update(|map| {
                        map.remove(&id);
                    });
                }
                GraphEvent::NodesDeleted { ids } => {
                    nodes.update(|ns| ns.retain(|n| !ids.contains(&n.id)));
                }
                GraphEvent::CreateNode {
                    item_id,
                    position,
                    connect_from,
                    connect_to,
                    connect_direction,
                } => {
                    let node_id = next_id(&item_id);
                    let cat = node_registry
                        .get(&item_id)
                        .and_then(|def| def.menu_item.category.clone());
                    let position_signal = app_owner.with(|| RwSignal::new(position));
                    nodes.update(|ns| {
                        ns.push(DynNode {
                            id: node_id.clone(),
                            node_type: item_id,
                            position: position_signal,
                            category: cat,
                        });
                    });

                    // Wire connection if menu was opened during a draft
                    if let (Some(draft_port), Some(new_port)) = (connect_from, connect_to) {
                        let new_port_id = format!("{}_{}", node_id, new_port);
                        let (source, target) = match connect_direction {
                            Some(PortDirection::Output) => (draft_port, new_port_id),
                            Some(PortDirection::Input) => (new_port_id, draft_port),
                            None => (draft_port, new_port_id),
                        };
                        connections.update(|map| {
                            let id = next_id("conn");
                            map.insert(id.clone(), ConnectionEntry { id, source, target });
                        });
                    }
                }
                GraphEvent::GroupCreated { node_ids } => {
                    if node_ids.len() > 1 {
                        let group_id = next_id("group");
                        groups.update(|gs| {
                            gs.push(GroupBox {
                                id: group_id,
                                node_ids,
                                label: Some("New Group".into()),
                                color: Some(random_group_color()),
                                error: false,
                            });
                        });
                    }
                }
                other => {
                    console::log_1(&format!("Unhandled! Graph event: {:?}", other).into());
                }
            }
        })
    };

    provide_context(ConnectionStyle {
        // stroke: "#71717a".into(),
        // stroke_selected: "#ef4444".into(),
        // stroke_draft: "#22d3ee".into(),
        // stroke_width: 2.0,
        // stroke_width_selected: 3.0,
        stroke: "#71717a".into(),
        stroke_selected: "#dddddd".into(),
        ..Default::default()
    });

    provide_context(SelectionBoxStyle {
        border: "1px solid rgba(255, 255, 255, 0.1)".into(),
        background: "rgba(255, 255, 255, 0.025)".into(),
    });

    provide_context(NodeStyle {
        header_padding_y: 4.0,
        body_padding_y: 2.0,
        border_radius: "0.125rem".into(),
        header_accent_height: 2.0,
        outline_selected: "1px solid red".into(),
        border: "none".into(),
        background: "#111111".into(),
        header_background: "#111111".into(),
        header_border_bottom: "none".into(),
        body_border_bottom: "none".into(),
        ..Default::default()
    });

    provide_context(theme::AnchorStyle {
        row_height: 20.0,
        ..Default::default()
    });

    let groups_signal = Signal::derive(move || groups.get());

    let on_group_event = Callback::new(move |event: GroupEvent<String>| match event {
        GroupEvent::Renamed {
            group_id,
            new_label,
        } => {
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    g.label = Some(new_label);
                }
            });
        }
        GroupEvent::MembersChanged { group_id, node_ids } => {
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    g.node_ids = node_ids;
                }
            });
        }
        GroupEvent::ColorChanged {
            group_id,
            new_color,
        } => {
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    g.color = Some(new_color);
                }
            });
        }
    });

    view! {
        <style>"html, body { margin: 0; padding: 0; background: #18181b; color-scheme: dark; }"</style>
        <div
            data-drop-target=""
            style="width: 100vw; height: 100vh; overflow: hidden; \
                     font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, sans-serif; \
                     font-size: 13px; color: #d4d4d8; box-sizing: border-box;"
            // Drop handling lives on the WRAPPER, outside the editor, which is
            // where a cross-pane drag has to be caught. The editor's context
            // isn't reachable from here — `EditorHandle` is.
            on:dragover=|ev: web_sys::DragEvent| ev.prevent_default()
            on:drop=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                let item_id = ev
                    .data_transfer()
                    .and_then(|dt| dt.get_data("text/plain").ok())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "color_source".to_string());
                let Some(position) = handle
                    .client_to_canvas(ev.client_x() as f64, ev.client_y() as f64)
                else {
                    return;
                };
                let node_id = next_id(&item_id);
                let cat = drop_registry
                    .get(&item_id)
                    .and_then(|def| def.menu_item.category.clone());
                let position_signal = drop_owner.with(|| RwSignal::new(position));
                nodes
                    .update(|ns| {
                        ns.push(DynNode {
                            id: node_id,
                            node_type: item_id,
                            position: position_signal,
                            category: cat,
                        });
                    });
            }
        >
            <NodeEditor
                handle=handle
                config={EditorConfig::default()}
                connections=connections_signal
                on_event=on_event
                _marker={PhantomData::<DemoPort>}
                menu_items=menu_items
                menu_search=menu_search
                groups=groups_signal
                on_group_event=on_group_event
            >
                <For
                    each=move || nodes.get()
                    key=|n| n.id.clone()
                    let:node
                >
                    <DynNodeView node=node registry=node_registry.clone() />
                </For>
            </NodeEditor>
        </div>
    }
}

/// Renders the appropriate node component based on node_type.
/// Renders a node using the type registry.
#[component]
fn DynNodeView(node: DynNode, registry: NodeTypeRegistry) -> impl IntoView {
    registry
        .render(&node.node_type, node.id, node.position)
        .unwrap_or_else(|| view! { <div>"Unknown node type"</div> }.into_any())
}

// Node components and registry are in nodes.rs
