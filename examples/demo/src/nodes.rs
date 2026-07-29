use leptos::prelude::*;
use leptos_node_graph::*;

use crate::utils::catalog::node_catalog;
use crate::widgets::{Select, NumberInput, options_from};

/// Context for Custom node's dynamic port counts.
#[derive(Clone, Copy)]
struct CustomPortCounts {
    inputs: Signal<usize>,
    outputs: Signal<usize>,
}

/// Provided by the Mix node's body and read from INSIDE its `NodeOverlay` — the
/// overlay is portalled to the pane, so this also demonstrates that a node
/// body's reactive context reaches its overlay content.
#[derive(Clone, Copy)]
struct BlendMix {
    amount: RwSignal<f64>,
}


/// Overlay content for the Mix node. Rendered at the graph-pane level, but it
/// still resolves `BlendMix` from the node body that opened it.
#[component]
fn MixAmountPanel() -> impl IntoView {
    let mix = use_context::<BlendMix>();
    view! {
        <div
            data-mix-panel=""
            style="background: #18181b; border: 1px solid #3f3f46; border-radius: 6px; \
                   box-shadow: 0 8px 24px rgba(0,0,0,0.5); padding: 10px; \
                   display: flex; flex-direction: column; gap: 8px;"
        >
            <div style="font-size: 10px; letter-spacing: 0.04em; text-transform: uppercase; color: #a1a1aa;">
                "Mix amount"
            </div>
            {match mix {
                Some(mix) => view! {
                    <input
                        type="range" min="0" max="1" step="0.01"
                        prop:value=move || mix.amount.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                mix.amount.set(v);
                            }
                        }
                    />
                    <div data-mix-value="" style="font-size: 11px; color: #d4d4d8;">
                        {move || format!("{:.2}", mix.amount.get())}
                    </div>
                }.into_any(),
                None => view! {
                    <div style="font-size: 11px; color: #ef4444;">"context lost"</div>
                }.into_any(),
            }}
        </div>
    }
}

// ---------------------------------------------------------------------------
// Registry builder
// ---------------------------------------------------------------------------

pub fn build_node_registry() -> NodeTypeRegistry {
    let mut reg = NodeTypeRegistry::new();

    // Global: all Float inputs get a NumberInput
    reg.register_port_type_slot(
        super::DemoPort::Float,
        PortDirection::Input,
        |label| {
            let value = RwSignal::new("0.0".to_string());
            view! { <NumberInput label=label value=value /> }.into_any()
        },
    );

    let catalog = node_catalog();

    for item in catalog {
        let type_id = item.id.clone();
        match type_id.as_str() {
            // Mix: has a blend dropdown body
            "mix" => {
                reg.register(
                    NodeTypeBuilder::<super::DemoPort>::new(item)
                        .body(|_node_id| {
                            let blend = RwSignal::new("Normal".to_string());
                            // Owned by the node body, edited from inside the overlay.
                            let mix = BlendMix { amount: RwSignal::new(0.5) };
                            provide_context(mix);
                            let open = RwSignal::new(false);
                            view! {
                                <NodeField label="Blend">
                                    <Select options=options_from(&["Normal", "Multiply", "Screen", "Overlay", "Add"]) value=blend />
                                    <button
                                        data-curve-trigger=""
                                        title="Edit mix amount"
                                        style="background: #27272a; border: 1px solid #3f3f46; \
                                               border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                                               padding: 2px 6px; cursor: pointer;"
                                        on:click=move |_| open.update(|o| *o = !*o)
                                    >
                                        "✎"
                                    </button>
                                </NodeField>
                                <Show when=move || open.get()>
                                    <NodeOverlay
                                        anchor=OverlayAnchor::Selector("[data-curve-trigger]".into())
                                        side=OverlaySide::Right
                                        on_dismiss=Callback::new(move |_| open.set(false))
                                        style="width: 200px;"
                                    >
                                        <MixAmountPanel />
                                    </NodeOverlay>
                                </Show>
                            }.into_any()
                        })
                        .build()
                );
            }
            // Custom: dynamic ports via signals
            "custom" => {
                reg.register(
                    NodeTypeBuilder::<super::DemoPort>::new(item)
                        .body(|_node_id| {
                            let num_in_str = RwSignal::new("2".to_string());
                            let num_out_str = RwSignal::new("1".to_string());
                            let num_in = Signal::derive(move || num_in_str.get().parse::<usize>().unwrap_or(2));
                            let num_out = Signal::derive(move || num_out_str.get().parse::<usize>().unwrap_or(1));
                            provide_context(CustomPortCounts { inputs: num_in, outputs: num_out });
                            let count_opts: Vec<(String, String)> = (0..=8).map(|n| (n.to_string(), n.to_string())).collect();
                            let co2 = count_opts.clone();
                            view! {
                                <div style="display: flex; flex-direction: column; gap: 6px;">
                                    <NodeField label="Inputs">
                                        <Select options=count_opts value=num_in_str />
                                    </NodeField>
                                    <NodeField label="Outputs">
                                        <Select options=co2 value=num_out_str />
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

// Widgets (Select, NumberInput) are in widgets.rs
// No manual node components needed — everything uses the builder.
