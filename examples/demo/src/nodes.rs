use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_node_graph::*;

use crate::utils::catalog::node_catalog;

/// Context for Custom node's dynamic port counts.
#[derive(Clone, Copy)]
struct CustomPortCounts {
    inputs: Signal<usize>,
    outputs: Signal<usize>,
}

// Type markers for the custom node component
type NodeM = PhantomData<(String, String, super::DemoPort)>;
type AnchorM = PhantomData<(String, String)>;
const NODE_MARKER: NodeM = PhantomData;
const ANCHOR_MARKER: AnchorM = PhantomData;

// ---------------------------------------------------------------------------
// Registry builder
// ---------------------------------------------------------------------------

pub fn build_node_registry() -> NodeTypeRegistry {
    let mut reg = NodeTypeRegistry::new();

    // Global: all Float inputs get a NumberInput
    reg.register_port_type_slot(
        super::DemoPort::Float,
        PortDirection::Input,
        |label| view! { <NumberInput label=label /> }.into_any(),
    );

    let catalog = node_catalog();

    for item in catalog {
        let type_id = item.id.clone();
        match type_id.as_str() {
            // Mix: has a blend dropdown body
            "mix" => {
                reg.register(
                    NodeTypeBuilder::<super::DemoPort>::new(item)
                        .body(|_node_id| view! {
                            <NodeField label="Blend">
                                <Select options=vec!["Normal", "Multiply", "Screen", "Overlay", "Add"] />
                            </NodeField>
                        }.into_any())
                        .build()
                );
            }
            // Custom: dynamic ports via signals
            "custom" => {
                reg.register(
                    NodeTypeBuilder::<super::DemoPort>::new(item)
                        .body(|_node_id| {
                            let (num_in, set_num_in) = signal(2usize);
                            let (num_out, set_num_out) = signal(1usize);
                            // Store in context so dynamic_inputs/outputs can read them
                            provide_context(CustomPortCounts { inputs: num_in.into(), outputs: num_out.into() });
                            view! {
                                <div style="display: flex; flex-direction: column; gap: 6px;">
                                    <NodeField label="Inputs">
                                        <select
                                            style="flex: 1; background: #27272a; border: 1px solid #3f3f46; \
                                                   border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                                                   padding: 3px 6px; outline: none; cursor: pointer;"
                                            on:change=move |ev| {
                                                use leptos::wasm_bindgen::JsCast;
                                                let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                                                if let Ok(n) = t.value().parse::<usize>() { set_num_in.set(n); }
                                            }
                                        >
                                            {(0..=8).map(|n| view! { <option value=n.to_string() selected={n == 2}>{n.to_string()}</option> }).collect_view()}
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
                                                if let Ok(n) = t.value().parse::<usize>() { set_num_out.set(n); }
                                            }
                                        >
                                            {(0..=8).map(|n| view! { <option value=n.to_string() selected={n == 1}>{n.to_string()}</option> }).collect_view()}
                                        </select>
                                    </NodeField>
                                </div>
                            }.into_any()
                        })
                        .dynamic_inputs(|_node_id| {
                            Signal::derive(move || {
                                let counts = use_context::<CustomPortCounts>();
                                let n = counts.map(|c| c.inputs.get()).unwrap_or(2);
                                (0..n).map(|i| TypedPort::input(
                                    format!("in_{i}"), format!("In {i}"), super::DemoPort::Any
                                )).collect()
                            })
                        })
                        .dynamic_outputs(|_node_id| {
                            Signal::derive(move || {
                                let counts = use_context::<CustomPortCounts>();
                                let n = counts.map(|c| c.outputs.get()).unwrap_or(1);
                                (0..n).map(|i| TypedPort::output(
                                    format!("out_{i}"), format!("Out {i}"), super::DemoPort::Any
                                )).collect()
                            })
                        })
                        .build()
                );
            }
            // Everything else: auto-render from definition
            _ => {
                reg.register(
                    NodeTypeBuilder::<super::DemoPort>::new(item).build()
                );
            }
        }
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

// No manual node components needed — everything is auto-rendered
// from TypedNodeDef + builder + global port type slots.

#[component]
fn _CustomNodeUnused(id: String, position: RwSignal<Position>) -> impl IntoView {
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
            inputs=ViewFn::from(move || {
                let n = num_inputs.get();
                let id = id_in.clone();
                (0..n).map(|i| {
                    let port_id = format!("{}_in_{}", id, i);
                    view! {
                        <InputAnchor id=port_id port_type=super::DemoPort::Any _marker=ANCHOR_MARKER label=format!("In {i}") />
                    }
                }).collect_view().into_any()
            })
            outputs=ViewFn::from(move || {
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
