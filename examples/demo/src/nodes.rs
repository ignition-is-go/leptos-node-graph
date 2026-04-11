use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_node_graph::*;

use crate::utils::catalog::node_catalog;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type M = PhantomData<(String, String, super::DemoPort)>;
type AM = PhantomData<(String, String)>;
const NODE_MARKER: M = PhantomData;
const ANCHOR_MARKER: AM = PhantomData;

// ---------------------------------------------------------------------------
// Registry builder
// ---------------------------------------------------------------------------

pub fn build_node_registry() -> NodeTypeRegistry {
    let mut reg = NodeTypeRegistry::new();
    let catalog = node_catalog();

    for item in catalog {
        let type_id = item.id.clone();
        reg.register(NodeTypeDef::new(item, move |id, pos| {
            match type_id.as_str() {
                "color_source" => view! { <ColorSourceNode id=id position=pos /> }.into_any(),
                "mix" => view! { <MixNode id=id position=pos /> }.into_any(),
                "math" => view! { <MathNode id=id position=pos /> }.into_any(),
                "output" => view! { <OutputNode id=id position=pos /> }.into_any(),
                "custom" => view! { <CustomNode id=id position=pos /> }.into_any(),
                _ => view! { <div>"Unknown"</div> }.into_any(),
            }
        }));
    }

    reg
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn header_view(title: &str) -> impl IntoView {
    let t = title.to_string();
    view! { {t} }
}

/// Inline number input for anchor slot content.
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

/// Styled select dropdown.
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
            outputs=Box::new(move || view! {
                <OutputAnchor id=color_id.clone() port_type=super::DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
                <OutputAnchor id=alpha_id.clone() port_type=super::DemoPort::Float _marker=ANCHOR_MARKER label="Alpha" />
            }.into_any())
        />
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
                <NodeField label="Blend">
                    <Select options=vec!["Normal", "Multiply", "Screen", "Overlay", "Add"] />
                </NodeField>
            }.into_any())
            inputs=Box::new(move || view! {
                <InputAnchor id=a_id.clone() port_type=super::DemoPort::Color _marker=ANCHOR_MARKER label="A" />
                <InputAnchor id=b_id.clone() port_type=super::DemoPort::Color _marker=ANCHOR_MARKER label="B" />
                <InputAnchor id=factor_id.clone() port_type=super::DemoPort::Float _marker=ANCHOR_MARKER>
                    <NumberInput label="Factor" initial="0.5" />
                </InputAnchor>
            }.into_any())
            outputs=Box::new(move || view! {
                <OutputAnchor id=result_id.clone() port_type=super::DemoPort::Color _marker=ANCHOR_MARKER label="Result" />
            }.into_any())
        />
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
            inputs=Box::new(move || view! {
                <InputAnchor id=a_id.clone() port_type=super::DemoPort::Float _marker=ANCHOR_MARKER>
                    <NumberInput label="A" />
                </InputAnchor>
                <InputAnchor id=b_id.clone() port_type=super::DemoPort::Float _marker=ANCHOR_MARKER>
                    <NumberInput label="B" />
                </InputAnchor>
            }.into_any())
            outputs=Box::new(move || view! {
                <OutputAnchor id=result_id.clone() port_type=super::DemoPort::Float _marker=ANCHOR_MARKER label="Result" />
            }.into_any())
        />
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
            inputs=Box::new(move || view! {
                <InputAnchor id=color_id.clone() port_type=super::DemoPort::Color _marker=ANCHOR_MARKER label="Color" />
                <InputAnchor id=value_id.clone() port_type=super::DemoPort::Any _marker=ANCHOR_MARKER label="Value" />
            }.into_any())
        />
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
                <div style="display: flex; flex-direction: column; gap: 6px;">
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
                </div>
            }.into_any())
            inputs=Box::new(move || {
                let n = num_inputs.get();
                let id = id_in.clone();
                (0..n).map(|i| {
                    let port_id = format!("{}_in_{}", id, i);
                    view! {
                        <InputAnchor id=port_id port_type=super::DemoPort::Any _marker=ANCHOR_MARKER label=format!("In {i}") />
                    }
                }).collect_view().into_any()
            })
            outputs=Box::new(move || {
                let n = num_outputs.get();
                let id = id_out.clone();
                (0..n).map(|i| {
                    let port_id = format!("{}_out_{}", id, i);
                    view! {
                        <OutputAnchor id=port_id port_type=super::DemoPort::Any _marker=ANCHOR_MARKER label=format!("Out {i}") />
                    }
                }).collect_view().into_any()
            })
        />
    }
}
