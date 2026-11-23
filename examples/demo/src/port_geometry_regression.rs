use crate::DemoPort;
use leptos::prelude::*;
use leptos_node_graph::*;
use std::{collections::HashMap, marker::PhantomData};

#[component]
pub fn PortGeometryRegression() -> impl IntoView {
    let position = RwSignal::new(Position::new(100.0, 100.0));
    let tall = RwSignal::new(false);
    let reverse = RwSignal::new(false);
    view! {
        <button id="move-node" on:click=move |_| position.update(|p| {p.x += 20.0; p.y += 10.0;})>"Move"</button>
        <button id="resize-body" on:click=move |_| tall.update(|v| *v = !*v)>"Resize"</button>
        <button id="reverse-ports" on:click=move |_| reverse.update(|v| *v = !*v)>"Reverse"</button>
        <div style="width:900px;height:600px">
            <NodeEditor
                config=EditorConfig::default()
                connections=Signal::derive(HashMap::<String, ConnectionEntry<String, String>>::new)
                on_event=Callback::new(|_: GraphEvent<String, String, String>| {})
                _marker={PhantomData::<DemoPort>}
            >
                <Node
                    id="node".to_string()
                    position=position
                    _marker={PhantomData::<(String, String, DemoPort)>}
                    header={Box::new(|| "Geometry".into_any())}
                    body={Box::new(move || view! {<div style:height=move || if tall.get() {"100px"} else {"20px"}>"Body"</div>}.into_any())}
                    inputs={move || view! {
                        <For each={move || if reverse.get() {vec!["b", "a"]} else {vec!["a", "b"]}} key=|id| *id let:id>
                            <InputAnchor<String, String, String, DemoPort> id=id.to_string() port_type=DemoPort::Float label=id.to_string() />
                        </For>
                    }}
                />
                <GeometryReadout />
            </NodeEditor>
        </div>
    }
}

#[component]
fn GeometryReadout() -> impl IntoView {
    let registry = expect_context::<EditorRegistry<String, String, String, DemoPort>>();
    view! {<pre id="geometry">{move || {
        registry.ports.with(|ports| {
            ["a", "b"].iter().filter_map(|id| ports.get(*id).map(|p| format!("{}:{},{}", id, p.position.x, p.position.y))).collect::<Vec<_>>().join(";")
        })
    }}</pre>}
}
