use std::marker::PhantomData;

use leptos::prelude::*;

use crate::registry::EditorRegistry;
use crate::types::*;

/// Style configuration for the selection box.
#[derive(Clone, Debug)]
pub struct SelectionBoxStyle {
    pub border: String,
    pub background: String,
}

impl Default for SelectionBoxStyle {
    fn default() -> Self {
        Self {
            border: "1px solid rgba(99, 102, 241, 0.6)".into(),
            background: "rgba(99, 102, 241, 0.1)".into(),
        }
    }
}

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
    let style_config = use_context::<SelectionBoxStyle>().unwrap_or_default();

    let box_style = move || {
        let bs = registry.box_select.get();
        let vp = registry.viewport.get();

        bs.map(|bs| {
            let rect = bs.to_rect();
            let screen_pos = vp.canvas_to_screen(rect.position);
            let width = rect.size.width * vp.zoom;
            let height = rect.size.height * vp.zoom;

            format!(
                "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px; \
                 border: {}; background: {}; pointer-events: none; z-index: 1000;",
                screen_pos.x,
                screen_pos.y,
                width,
                height,
                style_config.border,
                style_config.background
            )
        })
    };

    move || {
        box_style().map(|style| {
            view! { <div style=style /> }
        })
    }
}
