use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_node_graph::*;
use web_sys::console;

mod utils;
use utils::{catalog::node_catalog};

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

/// Card wrapper that reads NodeContext for selection/drag styling.
/// Applied inside the Node's render tree.
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

fn header_view(title: &str) -> impl IntoView {
    let t = title.to_string();
    view! {
        <div style="padding: 6px 12px; font-weight: 600; font-size: 12px; \
                     letter-spacing: 0.03em; text-transform: uppercase; color: #a1a1aa; \
                     border-bottom: 1px solid #27272a; border-radius: 8px 8px 0 0; \
                     background: #232327;">
            {t}
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
                            .is_some_and(|c| c.to_lowercase().contains(&search))
                        || item
                            .description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&search))
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
                if let Some(g) = gs.iter_mut().find(|g| g.id == group_id)
                    && !g.node_ids.contains(&node_id) {
                        g.node_ids.push(node_id);
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
        <Node id=id position=position _marker=NODE_MARKER
            header=Box::new(|| header_view("Color Source").into_any())
        >
            <OutputAnchor id=color_id port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
            <OutputAnchor id=alpha_id port_type=DemoPort::Float _marker=ANCHOR_MARKER label="Alpha" />
        </Node>
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
        <Node id=id position=position _marker=NODE_MARKER
            header=Box::new(|| header_view("Mix").into_any())
            body=Box::new(|| view! {
                <NodeContent>
                    <NodeField label="Blend">
                        <Select options=vec!["Normal", "Multiply", "Screen", "Overlay", "Add"] />
                    </NodeField>
                </NodeContent>
            }.into_any())
        >
            <InputAnchor id=a_id port_type=DemoPort::Color _marker=ANCHOR_MARKER label="A" />
            <InputAnchor id=b_id port_type=DemoPort::Color _marker=ANCHOR_MARKER label="B" />
            <InputAnchor id=factor_id port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                <NumberInput label="Factor" initial="0.5" />
            </InputAnchor>
            <OutputAnchor id=result_id port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Result" />
        </Node>
    }
}

#[component]
fn MathNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let a_id = p("a");
    let b_id = p("b");
    let result_id = p("result");
    view! {
        <Node id=id position=position _marker=NODE_MARKER
            header=Box::new(|| header_view("Math").into_any())
        >
            <InputAnchor id=a_id port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                <NumberInput label="A" />
            </InputAnchor>
            <InputAnchor id=b_id port_type=DemoPort::Float _marker=ANCHOR_MARKER>
                <NumberInput label="B" />
            </InputAnchor>
            <OutputAnchor id=result_id port_type=DemoPort::Float _marker=ANCHOR_MARKER label="Result" />
        </Node>
    }
}

#[component]
fn OutputNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let p = |name: &str| format!("{id}_{name}");
    let color_id = p("color");
    let value_id = p("value");
    view! {
        <Node id=id position=position _marker=NODE_MARKER
            header=Box::new(|| header_view("Output").into_any())
        >
            <InputAnchor id=color_id port_type=DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
            <InputAnchor id=value_id port_type=DemoPort::Any _marker=ANCHOR_MARKER label="Value" />
        </Node>
    }
}

#[component]
fn CustomNode(id: String, position: RwSignal<Position>) -> impl IntoView {
    let (num_inputs, set_num_inputs) = signal(2usize);
    let (num_outputs, set_num_outputs) = signal(1usize);
    let id_in = id.clone();
    let id_out = id.clone();

    view! {
        <Node id=id position=position _marker=NODE_MARKER
            header=Box::new(|| header_view("Custom").into_any())
            body=Box::new(move || view! {
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
            }.into_any())
        >
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
        </Node>
    }
}
