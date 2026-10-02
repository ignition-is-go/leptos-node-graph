//! Deterministic pane boundaries for the browser regression suite.
use leptos::prelude::*;
use leptos_node_graph::NodeMenuStyle;

#[component]
pub fn PopupRegression() -> impl IntoView {
    let mounted = RwSignal::new(true);
    provide_context(NodeMenuStyle {
        background: "rgb(31, 42, 53)".into(),
        ..Default::default()
    });
    view! {
        <button id="toggle-pane" on:click=move |_| mounted.update(|value| *value = !*value)>
            "Toggle scene pane"
        </button>
        <section id="scene-pane" style="position:absolute;left:40px;top:80px;width:480px;height:480px;overflow:hidden;isolation:isolate;transform:translateZ(0)">
            <Show when=move || mounted.get()>
                <super::App contained=true />
            </Show>
        </section>
        <section id="neighbor-pane" style="position:absolute;left:520px;top:80px;width:480px;height:480px;overflow:hidden;isolation:isolate;background:#454545">
            <button id="neighbor-control">"Value Tracks control"</button>
        </section>
    }
}
