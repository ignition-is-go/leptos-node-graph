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
fn StyledNode(
    id: String,
    position: RwSignal<Position>,
    children: Children,
) -> impl IntoView {
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

/// Styled anchor wrapper. Reads `AnchorContext` for state-driven styling.
#[component]
fn AnchorRow(children: Children) -> impl IntoView {
    let ctx = expect_context::<AnchorContext>();

    let style = move || {
        let incompat = ctx.is_incompatible.get();
        let is_output = ctx.direction == PortDirection::Output;

        let opacity = if incompat { "0.25" } else { "1" };
        let pointer = if incompat { "pointer-events: none;" } else { "" };
        let direction = if is_output { "flex-direction: row-reverse;" } else { "" };

        format!(
            "display: flex; align-items: center; gap: 6px; padding: 4px 10px; \
             cursor: crosshair; transition: opacity 0.15s; opacity: {opacity}; \
             {pointer} {direction}"
        )
    };

    view! { <div style=style>{children()}</div> }
}

/// The port dot circle. Reads `AnchorContext` for state-driven styling.
/// Attaches `dot_ref` from context so the library knows where to draw connections.
#[component]
fn AnchorDot() -> impl IntoView {
    let ctx = expect_context::<AnchorContext>();

    let style = move || {
        let compatible = ctx.is_compatible.get();
        let source = ctx.is_source.get();
        let connected = ctx.is_connected.get();

        let (border_color, bg) = if compatible || source {
            ("#22d3ee", "#22d3ee")
        } else if connected {
            ("#71717a", "#71717a")
        } else {
            ("#71717a", "transparent")
        };

        let shadow = if compatible {
            "box-shadow: 0 0 6px #22d3ee, 0 0 12px rgba(34,211,238,0.3);"
        } else {
            ""
        };

        format!(
            "width: 8px; height: 8px; border-radius: 50%; \
             border: 1.5px solid {border_color}; background: {bg}; \
             flex-shrink: 0; transition: all 0.15s; {shadow}"
        )
    };

    view! { <div style=style node_ref=ctx.dot_ref /> }
}

/// Anchor label. Highlights when compatible.
#[component]
fn AnchorLabel(#[prop(into)] text: String) -> impl IntoView {
    let ctx = expect_context::<AnchorContext>();

    let style = move || {
        let color = if ctx.is_compatible.get() { "#22d3ee" } else { "#a1a1aa" };
        format!("font-size: 11px; color: {color}; white-space: nowrap;")
    };

    view! { <span style=style>{text}</span> }
}

/// Input anchor with styled wrapper.
/// When children are provided, they're rendered inside the anchor (consumer must include AnchorRow + AnchorDot).
/// When no children, a default AnchorRow with dot + label is rendered.
#[component]
fn In(
    id: String,
    port_type: DemoPort,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    match children {
        Some(children) => view! {
            <InputAnchor id=id port_type=port_type _marker=ANCHOR_MARKER>
                {children()}
            </InputAnchor>
        }.into_any(),
        None => {
            let lbl = label.unwrap_or_default();
            view! {
                <InputAnchor id=id port_type=port_type _marker=ANCHOR_MARKER>
                    <DefaultAnchorContent label=lbl />
                </InputAnchor>
            }.into_any()
        }
    }
}

/// Default anchor content — rendered inside the anchor so AnchorContext is available.
#[component]
fn DefaultAnchorContent(#[prop(into)] label: String) -> impl IntoView {
    view! {
        <AnchorRow>
            <AnchorDot />
            <AnchorLabel text=label />
        </AnchorRow>
    }
}

/// Output anchor with styled wrapper.
#[component]
fn Out(
    id: String,
    port_type: DemoPort,
    #[prop(into)] label: String,
) -> impl IntoView {
    view! {
        <OutputAnchor id=id port_type=port_type _marker=ANCHOR_MARKER>
            <DefaultAnchorContent label=label />
        </OutputAnchor>
    }
}

#[component]
fn NumberInput(
    #[prop(into)] label: String,
    #[prop(optional, into)] initial: Option<String>,
) -> impl IntoView {
    let (value, set_value) = signal(initial.unwrap_or_else(|| "0.0".into()));
    view! {
        <div style="display: flex; align-items: center; gap: 4px; flex: 1; min-width: 0;">
            <AnchorLabel text=label />
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
                MenuPort { id: "color".into(), label: "Color".into(), direction: PortDirection::Output },
                MenuPort { id: "alpha".into(), label: "Alpha".into(), direction: PortDirection::Output },
            ],
        },
        NodeMenuItem {
            id: "mix".into(),
            label: "Mix".into(),
            category: Some("Color".into()),
            description: Some("Blend two colors".into()),
            ports: vec![
                MenuPort { id: "a".into(), label: "A".into(), direction: PortDirection::Input },
                MenuPort { id: "b".into(), label: "B".into(), direction: PortDirection::Input },
                MenuPort { id: "factor".into(), label: "Factor".into(), direction: PortDirection::Input },
                MenuPort { id: "result".into(), label: "Result".into(), direction: PortDirection::Output },
            ],
        },
        NodeMenuItem {
            id: "math".into(),
            label: "Math".into(),
            category: Some("Math".into()),
            description: Some("Arithmetic operation".into()),
            ports: vec![
                MenuPort { id: "a".into(), label: "A".into(), direction: PortDirection::Input },
                MenuPort { id: "b".into(), label: "B".into(), direction: PortDirection::Input },
                MenuPort { id: "result".into(), label: "Result".into(), direction: PortDirection::Output },
            ],
        },
        NodeMenuItem {
            id: "output".into(),
            label: "Output".into(),
            category: Some("Output".into()),
            description: Some("Final output destination".into()),
            ports: vec![
                MenuPort { id: "color".into(), label: "Color".into(), direction: PortDirection::Input },
                MenuPort { id: "value".into(), label: "Value".into(), direction: PortDirection::Input },
            ],
        },
    ]
}

#[component]
fn App() -> impl IntoView {
    let connections: RwSignal<HashMap<String, ConnectionEntry<String, String>>> =
        RwSignal::new(HashMap::new());
    let connections_signal = Signal::derive(move || connections.get());

    // Dynamic node list
    let nodes: RwSignal<Vec<DynNode>> = RwSignal::new(vec![
        DynNode { id: "color_source_0".into(), node_type: "color_source".into(), position: RwSignal::new(Position::new(50.0, 100.0)) },
        DynNode { id: "mix_0".into(), node_type: "mix".into(), position: RwSignal::new(Position::new(300.0, 200.0)) },
        DynNode { id: "math_0".into(), node_type: "math".into(), position: RwSignal::new(Position::new(350.0, 10.0)) },
        DynNode { id: "output_0".into(), node_type: "output".into(), position: RwSignal::new(Position::new(650.0, 100.0)) },
    ]);

    // Filtered catalog based on search
    let menu_search: RwSignal<String> = RwSignal::new(String::new());
    let catalog = node_catalog();
    let menu_items = Signal::derive(move || {
        let search = menu_search.get().to_lowercase();
        let items = catalog.clone();
        if search.is_empty() {
            items
        } else {
            items.into_iter().filter(|item| {
                item.label.to_lowercase().contains(&search)
                    || item.category.as_ref().map_or(false, |c| c.to_lowercase().contains(&search))
                    || item.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&search))
            }).collect()
        }
    });

    let on_event = {
        let connections = connections;
        Callback::new(move |event: GraphEvent<String, String, String>| match event {
            GraphEvent::ConnectionRequested { source, target } => {
                let already =
                    connections.with_untracked(|c| c.values().any(|e| e.target == target));
                if already { return; }
                connections.update(|map| {
                    let id = next_id("conn");
                    map.insert(id.clone(), ConnectionEntry { id, source, target });
                });
            }
            GraphEvent::ConnectionRemoved { id } => {
                connections.update(|map| { map.remove(&id); });
            }
            GraphEvent::NodesDeleted { ids } => {
                nodes.update(|ns| ns.retain(|n| !ids.contains(&n.id)));
            }
            GraphEvent::CreateNode { item_id, position, connect_from, connect_to, connect_direction } => {
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
            other => {
                console::log_1(&format!("Graph event: {:?}", other).into());
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

    let groups: RwSignal<Vec<GroupBox<String>>> = RwSignal::new(vec![
        GroupBox {
            id: "color-processing".into(),
            node_ids: vec!["color_source_0".into(), "mix_0".into()],
            label: Some("Color Processing".into()),
            color: Some("#8b5cf6".into()),
            error: false,
        },
        GroupBox {
            id: "output-chain".into(),
            node_ids: vec!["math_0".into(), "output_0".into()],
            label: Some("Output".into()),
            color: Some("#22d3ee".into()),
            error: false,
        },
    ]);
    let groups_signal = Signal::derive(move || groups.get());

    let on_group_event = Callback::new(move |event: GroupEvent<String>| {
        match event {
            GroupEvent::Renamed { group_id, new_label } => {
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
            >
                <GroupBoxOverlay<String, String, String, DemoPort>
                    groups=groups_signal
                    on_event=on_group_event
                />
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
                    <Out id=color_id.clone() port_type=DemoPort::Color label="Color" />
                    <Out id=alpha_id.clone() port_type=DemoPort::Float label="Alpha" />
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
                    <In id=a_id.clone() port_type=DemoPort::Color label="A" />
                    <In id=b_id.clone() port_type=DemoPort::Color label="B" />
                    <In id=factor_id.clone() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="Factor" initial="0.5" />
                        </AnchorRow>
                    </In>
                }.into_any())
                outputs=Box::new(move || view! {
                    <Out id=result_id.clone() port_type=DemoPort::Color label="Result" />
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
                    <In id=a_id.clone() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="A" />
                        </AnchorRow>
                    </In>
                    <In id=b_id.clone() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="B" />
                        </AnchorRow>
                    </In>
                }.into_any())
                outputs=Box::new(move || view! {
                    <Out id=result_id.clone() port_type=DemoPort::Float label="Result" />
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
                    <In id=color_id.clone() port_type=DemoPort::Color label="Color" />
                    <In id=value_id.clone() port_type=DemoPort::Any label="Value" />
                }.into_any())
            />
        </StyledNode>
    }
}
