use leptos::wasm_bindgen::prelude::*;

pub fn request_animation_frame(f: impl FnOnce() + 'static) {
    let cb = Closure::once_into_js(f);
    let _ = web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}
