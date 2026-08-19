use leptos::prelude::*;

/// A styled select dropdown.
#[component]
pub fn Select(options: Vec<(String, String)>, value: RwSignal<String>) -> impl IntoView {
    view! {
        <select
            style="flex: 1; background: #27272a; border: 1px solid #3f3f46; \
                   border-radius: 4px; color: #d4d4d8; font-size: 11px; \
                   padding: 3px 6px; outline: none; cursor: pointer;"
            on:change=move |ev| {
                use leptos::wasm_bindgen::JsCast;
                let t = ev.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                value.set(t.value());
            }
            on:keydown=move |ev: web_sys::KeyboardEvent| { ev.stop_propagation(); }
        >
            {options.into_iter().map(|(val, label)| {
                let selected = value.get_untracked() == val;
                view! { <option value=val selected=selected>{label}</option> }
            }).collect_view()}
        </select>
    }
}

pub fn options_from(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|s| (s.to_string(), s.to_string()))
        .collect()
}

/// A styled number input for anchor slot content.
#[component]
pub fn NumberInput(#[prop(into)] label: String, value: RwSignal<String>) -> impl IntoView {
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
                    value.set(t.value());
                }
                on:mousedown=move |ev: web_sys::MouseEvent| { ev.stop_propagation(); }
                on:keydown=move |ev: web_sys::KeyboardEvent| { ev.stop_propagation(); }
            />
        </div>
    }
}
