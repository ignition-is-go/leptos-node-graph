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

#[component]
fn App() -> impl IntoView {
    let connections: RwSignal<HashMap<String, ConnectionEntry<String, String>>> =
        RwSignal::new(HashMap::new());
    let connections_signal = Signal::derive(move || connections.get());

    let color_source_pos = RwSignal::new(Position::new(50.0, 100.0));
    let mix_pos = RwSignal::new(Position::new(300.0, 200.0));
    let math_pos = RwSignal::new(Position::new(350.0, 10.0));
    let output_pos = RwSignal::new(Position::new(650.0, 100.0));

    let on_event = Callback::new(move |event: GraphEvent<String, String, String>| match event {
        GraphEvent::ConnectionRequested { source, target } => {
            let already =
                connections.with_untracked(|c| c.values().any(|e| e.target == target));
            if already { return; }
            connections.update(|map| {
                let id = format!("conn_{}", map.len());
                map.insert(id.clone(), ConnectionEntry { id: id.clone(), source, target });
            });
        }
        GraphEvent::ConnectionRemoved { id } => {
            connections.update(|map| { map.remove(&id); });
        }
        other => {
            console::log_1(&format!("Graph event: {:?}", other).into());
        }
    });

    // Provide connection style via context (picked up by ConnectionRenderer)
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
            >
                <ColorSourceNode position=color_source_pos />
                <MixNode position=mix_pos />
                <MathNode position=math_pos />
                <OutputNode position=output_pos />
            </NodeEditor>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Node components
// ---------------------------------------------------------------------------

#[component]
fn ColorSourceNode(position: RwSignal<Position>) -> impl IntoView {
    view! {
        <StyledNode id="color_source".into() position=position>
            <NodeHeader title="Color Source" />
            <NodePorts
                outputs=Box::new(|| view! {
                    <Out id="cs_color".into() port_type=DemoPort::Color label="Color" />
                    <Out id="cs_alpha".into() port_type=DemoPort::Float label="Alpha" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn MixNode(position: RwSignal<Position>) -> impl IntoView {
    view! {
        <StyledNode id="mix".into() position=position>
            <NodeHeader title="Mix" />
            <NodeContent>
                <NodeField label="Blend">
                    <Select options=vec!["Normal", "Multiply", "Screen", "Overlay", "Add"] />
                </NodeField>
            </NodeContent>
            <NodePorts
                inputs=Box::new(|| view! {
                    <In id="mix_a".into() port_type=DemoPort::Color label="A" />
                    <In id="mix_b".into() port_type=DemoPort::Color label="B" />
                    <In id="mix_factor".into() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="Factor" initial="0.5" />
                        </AnchorRow>
                    </In>
                }.into_any())
                outputs=Box::new(|| view! {
                    <Out id="mix_out".into() port_type=DemoPort::Color label="Result" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn MathNode(position: RwSignal<Position>) -> impl IntoView {
    view! {
        <StyledNode id="math".into() position=position>
            <NodeHeader title="Math" />
            <NodePorts
                inputs=Box::new(|| view! {
                    <In id="math_a".into() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="A" />
                        </AnchorRow>
                    </In>
                    <In id="math_b".into() port_type=DemoPort::Float>
                        <AnchorRow>
                            <AnchorDot />
                            <NumberInput label="B" />
                        </AnchorRow>
                    </In>
                }.into_any())
                outputs=Box::new(|| view! {
                    <Out id="math_result".into() port_type=DemoPort::Float label="Result" />
                }.into_any())
            />
        </StyledNode>
    }
}

#[component]
fn OutputNode(position: RwSignal<Position>) -> impl IntoView {
    view! {
        <StyledNode id="output".into() position=position>
            <NodeHeader title="Output" />
            <NodePorts
                inputs=Box::new(|| view! {
                    <In id="out_color".into() port_type=DemoPort::Color label="Color" />
                    <In id="out_value".into() port_type=DemoPort::Any label="Value" />
                }.into_any())
            />
        </StyledNode>
    }
}
