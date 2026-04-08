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
}

// ---------------------------------------------------------------------------
// Node descriptor
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DemoNode {
    id: String,
    label: String,
    position: RwSignal<Position>,
    inputs: Vec<(String, String, DemoPort)>,
    outputs: Vec<(String, String, DemoPort)>,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let nodes = RwSignal::new(vec![
        DemoNode {
            id: "color_source".into(),
            label: "Color Source".into(),
            position: RwSignal::new(Position::new(50.0, 100.0)),
            inputs: vec![],
            outputs: vec![
                ("cs_color".into(), "Color".into(), DemoPort::Color),
                ("cs_alpha".into(), "Alpha".into(), DemoPort::Float),
            ],
        },
        DemoNode {
            id: "math".into(),
            label: "Math".into(),
            position: RwSignal::new(Position::new(350.0, 50.0)),
            inputs: vec![
                ("math_a".into(), "A".into(), DemoPort::Float),
                ("math_b".into(), "B".into(), DemoPort::Float),
            ],
            outputs: vec![("math_result".into(), "Result".into(), DemoPort::Float)],
        },
        DemoNode {
            id: "output".into(),
            label: "Output".into(),
            position: RwSignal::new(Position::new(650.0, 100.0)),
            inputs: vec![
                ("out_color".into(), "Color".into(), DemoPort::Color),
                ("out_value".into(), "Value".into(), DemoPort::Any),
            ],
            outputs: vec![],
        },
    ]);

    let connections: RwSignal<HashMap<String, ConnectionEntry<String, String>>> =
        RwSignal::new(HashMap::new());

    let connections_signal = Signal::derive(move || connections.get());

    let on_event = {
        let connections = connections;
        let nodes = nodes;
        Callback::new(move |event: GraphEvent<String, String, String>| match event {
            GraphEvent::NodeMoved { id, position } => {
                nodes.with_untracked(|nodes| {
                    if let Some(n) = nodes.iter().find(|n| n.id == id) {
                        n.position.set(position);
                    }
                });
            }
            GraphEvent::ConnectionRequested { source, target } => {
                let already =
                    connections.with_untracked(|c| c.values().any(|e| e.target == target));
                if already {
                    return;
                }
                connections.update(|map| {
                    let id = format!("conn_{}", map.len());
                    map.insert(
                        id.clone(),
                        ConnectionEntry {
                            id: id.clone(),
                            source,
                            target,
                        },
                    );
                });
            }
            GraphEvent::ConnectionRemoved { id } => {
                connections.update(|map| {
                    map.remove(&id);
                });
            }
            GraphEvent::NodesDeleted { ids } => {
                console::log_1(&format!("Nodes deleted: {:?}", ids).into());
                nodes.update(|node_list| {
                    node_list.retain(|n| !ids.contains(&n.id));
                });
            }
            other => {
                console::log_1(&format!("Graph event: {:?}", other).into());
            }
        })
    };

    let editor_marker: PhantomData<DemoPort> = PhantomData;

    view! {
        <Style />
        <NodeEditor
            config={EditorConfig::default()}
            connections=connections_signal
            on_event=on_event
            _marker=editor_marker
        >
            <For
                each=move || nodes.get()
                key=|node| node.id.clone()
                let:node
            >
                <DemoNodeView node=node />
            </For>
        </NodeEditor>
    }
}

#[component]
fn DemoNodeView(node: DemoNode) -> impl IntoView {
    let label = node.label.clone();
    let inputs = node.inputs.clone();
    let outputs = node.outputs.clone();
    let node_marker: PhantomData<(String, String, DemoPort)> = PhantomData;
    let anchor_marker: PhantomData<(String, String)> = PhantomData;

    view! {
        <Node
            id={node.id.clone()}
            position=node.position
            _marker=node_marker
        >
            <div class="node__header">{label}</div>
            <div class="node__body">
                <div class="node__inputs">
                    {inputs
                        .into_iter()
                        .map(|(id, label, port_type)| {
                            let m = anchor_marker;
                            view! {
                                <InputAnchor
                                    id=id
                                    port_type=port_type
                                    label=label
                                    _marker=m
                                />
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
                <div class="node__outputs">
                    {outputs
                        .into_iter()
                        .map(|(id, label, port_type)| {
                            let m = anchor_marker;
                            view! {
                                <OutputAnchor
                                    id=id
                                    port_type=port_type
                                    label=label
                                    _marker=m
                                />
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
            </div>
        </Node>
    }
}

#[component]
fn Style() -> impl IntoView {
    view! {
        <style>
            r#"
*, *::before, *::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

html, body {
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, sans-serif;
    font-size: 13px;
    color: #d4d4d8;
}

.node-editor {
    width: 100vw;
    height: 100vh;
    background-color: #18181b;
    background-image:
        radial-gradient(circle, #27272a 1px, transparent 1px);
    background-size: 24px 24px;
}

/* ---- Node ---- */
.node {
    background: #1e1e22;
    border: 1px solid #3f3f46;
    border-radius: 8px;
    min-width: 160px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    user-select: none;
}

.node--selected {
    border-color: #ef4444;
    box-shadow: 0 0 0 1px #ef4444, 0 4px 16px rgba(239, 68, 68, 0.25);
}

.node--dragging {
    opacity: 0.92;
}

.node__header {
    padding: 6px 12px;
    font-weight: 600;
    font-size: 12px;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: #a1a1aa;
    border-bottom: 1px solid #27272a;
    border-radius: 8px 8px 0 0;
    background: #232327;
}

.node__body {
    display: flex;
    justify-content: space-between;
    gap: 24px;
    padding: 8px 0;
}

.node__inputs,
.node__outputs {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.node__outputs {
    align-items: flex-end;
}

/* ---- Anchor ---- */
.anchor {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    cursor: crosshair;
    transition: background 0.15s;
}

.anchor:hover {
    background: rgba(255, 255, 255, 0.04);
}

.anchor--output {
    flex-direction: row-reverse;
}

.anchor__dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid #71717a;
    background: transparent;
    flex-shrink: 0;
    transition: all 0.15s;
}

.anchor--connected .anchor__dot {
    background: #71717a;
}

.anchor--compatible .anchor__dot {
    border-color: #22d3ee;
    background: #22d3ee;
    box-shadow: 0 0 6px #22d3ee, 0 0 12px rgba(34, 211, 238, 0.3);
    animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
    0%, 100% { box-shadow: 0 0 6px #22d3ee, 0 0 12px rgba(34, 211, 238, 0.3); }
    50%      { box-shadow: 0 0 10px #22d3ee, 0 0 20px rgba(34, 211, 238, 0.5); }
}

.anchor__label {
    font-size: 11px;
    color: #a1a1aa;
    white-space: nowrap;
}

/* ---- Connection (SVG) ---- */
.connection {
    stroke: #71717a;
    stroke-width: 2;
    fill: none;
}

.connection--selected {
    stroke: #ef4444;
    stroke-width: 3;
}

.connection--draft {
    stroke: #22d3ee;
    stroke-width: 2;
    stroke-dasharray: 6 4;
}

/* ---- Selection Box ---- */
.selection-box {
    border: 1px solid rgba(99, 102, 241, 0.6);
    background: rgba(99, 102, 241, 0.1);
    pointer-events: none;
}
            "#
        </style>
    }
}
