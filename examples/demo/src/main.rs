use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_node_graph::*;
use web_sys::console;

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

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type M = PhantomData<(String, String, DemoPort)>;
type AM = PhantomData<(String, String)>;

const NODE_MARKER: M = PhantomData;
const ANCHOR_MARKER: AM = PhantomData;

// ---------------------------------------------------------------------------
// Styled wrappers — all visual styles inline, driven by context signals
// ---------------------------------------------------------------------------

#[component]
fn StyledNode(id: String, position: RwSignal<Position>, children: Children) -> impl IntoView {
    view! {
        <Node id=id position=position _marker=NODE_MARKER>
            <NodeCard>{children()}</NodeCard>
        </Node>
    }
}

/// The visual card wrapper. Reads `NodeContext` for state-driven styling.
#[component]
fn NodeCard(children: Children) -> impl IntoView {
    let ctx = expect_context::<NodeContext<String>>();

    let style = move || {
        let selected = ctx.is_selected.get();
        let dragging = ctx.is_dragging.get();

        let border_color = if selected { "#ef4444" } else { "#3f3f46" };
        let shadow = if selected {
            "0 0 0 1px #ef4444, 0 4px 16px rgba(239,68,68,0.25)"
        } else {
            "0 4px 12px rgba(0,0,0,0.4)"
        };
        let opacity = if dragging { "0.92" } else { "1" };

        format!(
            "background: #1e1e22; border: 1px solid {border_color}; border-radius: 8px; \
             min-width: 160px; box-shadow: {shadow}; user-select: none; opacity: {opacity};"
        )
    };

    view! { <div style=style>{children()}</div> }
}

#[component]
fn NodeHeader(#[prop(into)] title: String) -> impl IntoView {
    view! {
        <div style="padding: 6px 12px; font-weight: 600; font-size: 12px; \
                     letter-spacing: 0.03em; text-transform: uppercase; color: #a1a1aa; \
                     border-bottom: 1px solid #27272a; border-radius: 8px 8px 0 0; \
                     background: #232327;">
            {title}
        </div>
    }
}

#[component]
fn NodePorts(
    #[prop(optional)] inputs: Option<Children>,
    #[prop(optional)] outputs: Option<Children>,
) -> impl IntoView {
    view! {
        <div style="display: flex; justify-content: space-between; gap: 24px; padding: 8px 0;">
            <div style="display: flex; flex-direction: column; gap: 2px;">
                {inputs.map(|c| c())}
            </div>
            <div style="display: flex; flex-direction: column; gap: 2px; align-items: flex-end;">
                {outputs.map(|c| c())}
            </div>
        </div>
    }
}

#[component]
fn NodeContent(children: Children) -> impl IntoView {
    view! {
        <div style="padding: 6px 10px; border-bottom: 1px solid #27272a; \
                     display: flex; flex-direction: column; gap: 6px;">
            {children()}
        </div>
    }
}

#[component]
fn NodeField(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <div style="display: flex; align-items: center; gap: 6px;">
            <label style="font-size: 10px; color: #71717a; text-transform: uppercase; \
                          letter-spacing: 0.04em; min-width: 38px;">
                {label}
            </label>
            {children()}
        </div>
    }
}

// The library renders dot + tooltip + label by default.
// Consumer only provides children to override the label area.

#[component]
fn NumberInput(
    #[prop(into)] label: String,
    #[prop(optional, into)] initial: Option<String>,
) -> impl IntoView {
    let (value, set_value) = signal(initial.unwrap_or_else(|| "0.0".into()));
    view! {
        <div style="display: flex; align-items: center; gap: 4px; flex: 1; min-width: 0;">
            <span style="font-size: 11px; color: #a1a1aa; white-space: nowrap;">{label}</span>
            <input
                type="text"
                inputmode="decimal"
                style="width: 52px; background: #27272a; border: 1px solid #3f3f46; \
                       border-radius: 4px; color: #d4d4d8; font-size: 11px; padding: 2px 6px; \
                       outline: none; font-variant-numeric: tabular-nums; text-align: right;"
                prop:value=move || value.get()
                on:input=move |ev| {
                    use leptos::wasm_bindgen::JsCast;
                    let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>();
                    set_value.set(t.value());
                }
                on:mousedown=move |ev: web_sys::MouseEvent| { ev.stop_propagation(); }
            />
        </div>
    }
}

#[component]
fn Select(
    options: Vec<&'static str>,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    view! {
        <select
            style="flex: 1; background: #27272a; border: 1px solid #3f3f46; \
                   border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                   padding: 3px 6px; outline: none; cursor: pointer;"
            on:change=move |ev| {
                use leptos::wasm_bindgen::JsCast;
                let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                if let Some(cb) = &on_change { cb.run(t.value()); }
            }
        >
            {options.into_iter().enumerate().map(|(i, opt)| {
                view! { <option selected={i == 0}>{opt}</option> }
            }).collect_view()}
        </select>
    }
}

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
}

static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(100);

const GROUP_COLORS: &[&str] = &[
    "#8b5cf6", "#22d3ee", "#f59e0b", "#10b981", "#ef4444", "#ec4899", "#6366f1", "#14b8a6",
];

fn random_group_color() -> String {
    let n = NEXT_ID.load(std::sync::atomic::Ordering::Relaxed);
    GROUP_COLORS[n % GROUP_COLORS.len()].into()
}

/// Generate a demo graph with the given number of nodes, connections, and groups.
fn generate_demo_graph(
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
    let rows = (num_nodes + cols - 1) / cols;
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

fn next_id(prefix: &str) -> String {
    let n = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{prefix}_{n}")
}

/// The full node type catalog with port definitions.
fn node_catalog() -> Vec<NodeMenuItem> {
    vec![
        NodeMenuItem {
            id: "color_source".into(),
            label: "Color Source".into(),
            category: Some("Input".into()),
            description: Some("Produces a color and alpha".into()),
            ports: vec![
                MenuPort {
                    id: "color".into(),
                    label: "Color".into(),
                    direction: PortDirection::Output,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "alpha".into(),
                    label: "Alpha".into(),
                    direction: PortDirection::Output,
                    type_id: "Float".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "mix".into(),
            label: "Mix".into(),
            category: Some("Color".into()),
            description: Some("Blend two colors".into()),
            ports: vec![
                MenuPort {
                    id: "a".into(),
                    label: "A".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "b".into(),
                    label: "B".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "factor".into(),
                    label: "Factor".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "result".into(),
                    label: "Result".into(),
                    direction: PortDirection::Output,
                    type_id: "Color".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "math".into(),
            label: "Math".into(),
            category: Some("Math".into()),
            description: Some("Arithmetic operation".into()),
            ports: vec![
                MenuPort {
                    id: "a".into(),
                    label: "A".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "b".into(),
                    label: "B".into(),
                    direction: PortDirection::Input,
                    type_id: "Float".into(),
                },
                MenuPort {
                    id: "result".into(),
                    label: "Result".into(),
                    direction: PortDirection::Output,
                    type_id: "Float".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "output".into(),
            label: "Output".into(),
            category: Some("Output".into()),
            description: Some("Final output destination".into()),
            ports: vec![
                MenuPort {
                    id: "color".into(),
                    label: "Color".into(),
                    direction: PortDirection::Input,
                    type_id: "Color".into(),
                },
                MenuPort {
                    id: "value".into(),
                    label: "Value".into(),
                    direction: PortDirection::Input,
                    type_id: "Any".into(),
                },
            ],
        },
        NodeMenuItem {
            id: "custom".into(),
            label: "Custom".into(),
            category: Some("Utility".into()),
            description: Some("Configurable inputs/outputs".into()),
            ports: vec![
                MenuPort {
                    id: "in_0".into(),
                    label: "In 0".into(),
                    direction: PortDirection::Input,
                    type_id: "Any".into(),
                },
                MenuPort {
                    id: "out_0".into(),
                    label: "Out 0".into(),
                    direction: PortDirection::Output,
                    type_id: "Any".into(),
                },
            ],
        },
    ]
}

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

    // Filtered catalog based on search
    let menu_search: RwSignal<String> = RwSignal::new(String::new());
    let catalog = node_catalog();
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
                            .map_or(false, |c| c.to_lowercase().contains(&search))
                        || item
                            .description
                            .as_ref()
                            .map_or(false, |d| d.to_lowercase().contains(&search))
                })
                .collect()
        }
    });

    let groups: RwSignal<Vec<GroupBox<String>>> = RwSignal::new(initial_groups);

    let on_event = {
        let connections = connections;
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
                    nodes.update(|ns| {
                        ns.push(DynNode {
                            id: node_id.clone(),
                            node_type: item_id,
                            position: RwSignal::new(position),
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
        stroke: "#71717a".into(),
        stroke_selected: "#ef4444".into(),
        stroke_draft: "#22d3ee".into(),
        stroke_width: 2.0,
        stroke_width_selected: 3.0,
    });

    provide_context(SelectionBoxStyle {
        border: "1px solid rgba(99, 102, 241, 0.6)".into(),
        background: "rgba(99, 102, 241, 0.1)".into(),
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
        GroupEvent::NodeAdded { group_id, node_id } => {
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    if !g.node_ids.contains(&node_id) {
                        g.node_ids.push(node_id);
                    }
                }
            });
        }
        GroupEvent::NodeRemoved { group_id, node_id } => {
            groups.update(|gs| {
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id) {
                    g.node_ids.retain(|id| id != &node_id);
                }
            });
        }
    });

    view! {
        <style>"html, body { margin: 0; padding: 0; background: #18181b; color-scheme: dark; }"</style>
        <div style="width: 100vw; height: 100vh; overflow: hidden; \
                     font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, sans-serif; \
                     font-size: 13px; color: #d4d4d8; box-sizing: border-box;">
            <NodeEditor
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
                    <DynNodeView node=node />
                </For>
            </NodeEditor>
        </div>
    }
}

/// Renders the appropriate node component based on node_type.
#[component]
fn DynNodeView(node: DynNode) -> impl IntoView {
    let node_type = node.node_type.clone();
    let id = node.id.clone();
    let position = node.position;

    match node_type.as_str() {
        "color_source" => view! { <ColorSourceNode id=id position=position /> }.into_any(),
        "mix" => view! { <MixNode id=id position=position /> }.into_any(),
        "math" => view! { <MathNode id=id position=position /> }.into_any(),
        "output" => view! { <OutputNode id=id position=position /> }.into_any(),
        "custom" => view! { <CustomNode id=id position=position /> }.into_any(),
        _ => view! { <div>"Unknown node type"</div> }.into_any(),
    }
}

// ---------------------------------------------------------------------------
// Node components
// ---------------------------------------------------------------------------

#[component]
fn ColorSourceNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let color_id = p("color");
    let alpha_id = p("alpha");
    view! {
        <StyledNode id=id position=position>
            <NodeHeader title="Color Source" />
            <NodePorts
                outputs=Box::new(move || view! {
                    <OutputAnchor id=color_id.clone() port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
                    <OutputAnchor id=alpha_id.clone() port_type=DemoPort::Float _marker=ANCHOR_MARKER label="Alpha" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn MixNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let a_id = p("a");
    let b_id = p("b");
    let factor_id = p("factor");
    let result_id = p("result");
    view! {
        <StyledNode id=id position=position>
            <NodeHeader title="Mix" />
            <NodeContent>
                <NodeField label="Blend">
                    <Select options=vec!["Normal", "Multiply", "Screen", "Overlay", "Add"] />
                </NodeField>
            </NodeContent>
            <NodePorts
                inputs=Box::new(move || view! {
                    <InputAnchor id=a_id.clone() port_type=DemoPort::Color _marker=ANCHOR_MARKER label="A" />
                    <InputAnchor id=b_id.clone() port_type=DemoPort::Color _marker=ANCHOR_MARKER label="B" />
                    <InputAnchor id=factor_id.clone() port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                        <NumberInput label="Factor" initial="0.5" />
                    </InputAnchor>
                }.into_any())
                outputs=Box::new(move || view! {
                    <OutputAnchor id=result_id.clone() port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Result" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn MathNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let a_id = p("a");
    let b_id = p("b");
    let result_id = p("result");
    view! {
        <StyledNode id=id position=position>
            <NodeHeader title="Math" />
            <NodePorts
                inputs=Box::new(move || view! {
                    <InputAnchor id=a_id.clone() port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                        <NumberInput label="A" />
                    </InputAnchor>
                    <InputAnchor id=b_id.clone() port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                        <NumberInput label="B" />
                    </InputAnchor>
                }.into_any())
                outputs=Box::new(move || view! {
                    <OutputAnchor id=result_id.clone() port_type=DemoPort::Float _marker=ANCHOR_MARKER label="Result" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn OutputNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let color_id = p("color");
    let value_id = p("value");
    view! {
        <StyledNode id=id position=position>
            <NodeHeader title="Output" />
            <NodePorts
                inputs=Box::new(move || view! {
                    <InputAnchor id=color_id.clone() port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
                    <InputAnchor id=value_id.clone() port_type=DemoPort::Any _marker=ANCHOR_MARKER label="Value" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn CustomNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let (num_inputs, set_num_inputs) = signal(2usize);
    let (num_outputs, set_num_outputs) = signal(1usize);
    let id_in = id.clone();
    let id_out = id.clone();

    view! {
        <StyledNode id=id position=position>
            <NodeHeader title="Custom" />
            <NodeContent>
                <NodeField label="Inputs">
                    <select
                        style="flex: 1; background: #27272a; border: 1px solid #3f3f46; \
                               border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                               padding: 3px 6px; outline: none; cursor: pointer;"
                        on:change=move |ev| {
                            use leptos::wasm_bindgen::JsCast;
                            let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                            if let Ok(n) = t.value().parse::<usize>() {
                                set_num_inputs.set(n);
                            }
                        }
                    >
                        {(0..=8).map(|n| {
                            view! { <option value=n.to_string() selected={n == 2}>{n.to_string()}</option> }
                        }).collect_view()}
                    </select>
                </NodeField>
                <NodeField label="Outputs">
                    <select
                        style="flex: 1; background: #27272a; border: 1px solid #3f3f46; \
                               border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                               padding: 3px 6px; outline: none; cursor: pointer;"
                        on:change=move |ev| {
                            use leptos::wasm_bindgen::JsCast;
                            let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                            if let Ok(n) = t.value().parse::<usize>() {
                                set_num_outputs.set(n);
                            }
                        }
                    >
                        {(0..=8).map(|n| {
                            view! { <option value=n.to_string() selected={n == 1}>{n.to_string()}</option> }
                        }).collect_view()}
                    </select>
                </NodeField>
            </NodeContent>
            <div style="display: flex; justify-content: space-between; gap: 24px; padding: 8px 0;">
                <div style="display: flex; flex-direction: column; gap: 2px;">
                    {move || {
                        let n = num_inputs.get();
                        let id = id_in.clone();
                        (0..n).map(|i| {
                            let port_id = format!("{}_in_{}", id, i);
                            view! {
                                <InputAnchor id=port_id port_type=DemoPort::Any _marker=ANCHOR_MARKER label=format!("In {i}") />
                            }
                        }).collect_view()
                    }}
                </div>
                <div style="display: flex; flex-direction: column; gap: 2px; align-items: flex-end;">
                    {move || {
                        let n = num_outputs.get();
                        let id = id_out.clone();
                        (0..n).map(|i| {
                            let port_id = format!("{}_out_{}", id, i);
                            view! {
                                <OutputAnchor id=port_id port_type=DemoPort::Any _marker=ANCHOR_MARKER label=format!("Out {i}") />
                            }
                        }).collect_view()
                    }}
                </div>
            </div>
        </StyledNode>
    }
}
