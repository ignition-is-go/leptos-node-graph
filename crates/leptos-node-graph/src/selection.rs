use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::types::*;

#[component]
pub fn SelectionBox<N, P, C, T>(
    #[prop(optional)] _marker: PhantomData<(N, P, C, T)>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();

    let box_style = move || {
        let bs = registry.box_select.get();
        let vp = registry.viewport.get();

        bs.map(|bs| {
            let rect = bs.to_rect();
            // Convert canvas rect to screen coordinates
            let screen_pos = vp.canvas_to_screen(rect.position);
            let width = rect.size.width * vp.zoom;
            let height = rect.size.height * vp.zoom;

            format!(
                "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                 border: 1px dashed #4a9eff; background: rgba(74, 158, 255, 0.1); \
                 pointer-events: none; z-index: 1000;",
                screen_pos.x, screen_pos.y, width, height
            )
        })
    };

    move || {
        box_style().map(|style| {
            view! {
                <div class="selection-box" style=style />
            }
        })
    }
}
