//! Node-anchored overlays.
//!
//! A panel rendered inside a node body can't escape two things on its own:
//!
//! 1. The node's `overflow: hidden` clips any absolutely-positioned child to the
//!    node box.
//! 2. The canvas carries `transform: translate() scale()`, and a transformed
//!    ancestor is the containing block for `position: fixed` descendants — so a
//!    fixed panel is offset by the pan and scaled by the zoom instead of being
//!    positioned against the viewport. (At pan 0,0 / zoom 1 it looks correct,
//!    which is how this tends to survive review.)
//!
//! [`NodeOverlay`] solves both by portalling its children into a layer that
//! `NodeEditor` renders inside the graph pane but OUTSIDE the transformed
//! canvas. Content is therefore unscaled and positioned in pane space, while
//! still being clipped by the graph's own box rather than escaping to a global
//! layer.
//!
//! ```ignore
//! // In a node body — mount the overlay to open it, unmount to close.
//! let open = RwSignal::new(false);
//! view! {
//!     <button data-curve-trigger on:click=move |_| open.set(true)>"✎"</button>
//!     <Show when=move || open.get()>
//!         <NodeOverlay
//!             anchor=OverlayAnchor::Selector("[data-curve-trigger]".into())
//!             side=OverlaySide::Right
//!             on_dismiss=Callback::new(move |_| open.set(false))
//!         >
//!             <CurveEditor />
//!         </NodeOverlay>
//!     </Show>
//! }
//! ```

use std::sync::Arc;

use leptos::portal::Portal;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos_use::use_event_listener;

use crate::node::NodeElement;
use crate::types::{Rect, ViewportTransform};

/// Mount point for node-anchored overlays, provided by `NodeEditor`.
///
/// The mount element sits inside the graph pane but outside the transformed
/// canvas, so children render unscaled and are clipped at the pane boundary.
#[derive(Clone, Copy)]
pub struct NodeOverlayLayer {
    /// The layer element overlays are portalled into.
    pub mount: NodeRef<leptos::html::Div>,
    /// The live canvas transform. Overlays re-place when this changes, so they
    /// track their anchor across pan and zoom.
    pub viewport: Signal<ViewportTransform>,
}

/// What a [`NodeOverlay`] positions itself against.
#[derive(Clone, Default)]
pub enum OverlayAnchor {
    /// The enclosing node's root element. Requires being rendered inside a
    /// `Node` (which provides [`NodeElement`]).
    #[default]
    Node,
    /// An element within the enclosing node, by CSS selector — e.g. the trigger
    /// button that opened the overlay: `"[data-curve-trigger]"`.
    Selector(String),
    /// An element resolved on demand. Build with [`OverlayAnchor::element`].
    Element(Arc<dyn Fn() -> Option<web_sys::Element> + Send + Sync>),
    /// A fixed rect in pane space (pixels from the graph container's top-left).
    Pane(Rect),
}

impl OverlayAnchor {
    /// Anchor to whatever element a `NodeRef` holds, of any element type.
    pub fn element<E>(node_ref: NodeRef<E>) -> Self
    where
        E: leptos::html::ElementType + 'static,
        E::Output: JsCast + Clone + 'static,
    {
        Self::Element(Arc::new(move || {
            node_ref
                .get()
                .map(|el| el.unchecked_into::<web_sys::Element>())
        }))
    }
}

/// Which side of the anchor the overlay sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverlaySide {
    #[default]
    Right,
    Left,
    Top,
    Bottom,
}

/// How the overlay lines up along the anchor's cross axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverlayAlign {
    #[default]
    Start,
    Center,
    End,
}

/// A rect in pane space: pixels from the graph container's top-left, unscaled.
#[derive(Clone, Copy, Debug, Default)]
struct PaneRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// A panel anchored to a node (or to an element inside one), rendered at the
/// graph-pane level so it escapes the node's clipping and the canvas transform.
///
/// Mount it to open and unmount it to close — there is no `open` prop, so the
/// caller keeps full control (and the children's reactive owner is only alive
/// while the panel is). `on_dismiss` fires on a pointerdown outside the panel
/// and on Escape; acting on it is the caller's choice.
///
/// See the [module docs](self) for why this exists.
#[component]
pub fn NodeOverlay(
    /// What to position against. Defaults to the enclosing node.
    #[prop(optional)]
    anchor: OverlayAnchor,
    #[prop(optional)] side: OverlaySide,
    #[prop(optional)] align: OverlayAlign,
    /// Gap between anchor and panel, in unscaled pixels.
    #[prop(default = 8.0)]
    offset: f64,
    /// Flip to the opposite side when the panel doesn't fit, then shift it to
    /// stay inside the pane.
    #[prop(default = true)]
    keep_in_view: bool,
    /// Re-place every frame, so the panel follows its anchor through node drags
    /// and content reflow. Pan and zoom are tracked either way.
    #[prop(default = true)]
    track_anchor: bool,
    /// Render a transparent click-catching backdrop over the pane, which fires
    /// `on_dismiss` when clicked.
    #[prop(optional)]
    backdrop: bool,
    /// Fired on a pointerdown outside the panel, and on Escape.
    #[prop(optional, into)]
    on_dismiss: Option<Callback<()>>,
    /// Extra style on the panel wrapper (e.g. `"width: 320px;"`).
    #[prop(optional, into)]
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let Some(layer) = use_context::<NodeOverlayLayer>() else {
        leptos::logging::warn!(
            "<NodeOverlay> rendered outside a <NodeEditor>; nothing will be shown"
        );
        return ().into_any();
    };
    let node_el = use_context::<NodeElement>();

    let panel_ref = NodeRef::<leptos::html::Div>::new();
    // `None` until the first placement, so the panel never flashes at 0,0.
    let placement = RwSignal::new(None::<(f64, f64)>);

    let anchor = Arc::new(anchor);
    let anchor_for_place = Arc::clone(&anchor);
    let extra_style = style.unwrap_or_default();

    // Resolve the anchor to a rect in pane space.
    let resolve_anchor = move |layer_el: &web_sys::Element| -> Option<PaneRect> {
        let lr = layer_el.get_bounding_client_rect();
        let from_element = |el: web_sys::Element| {
            let r = el.get_bounding_client_rect();
            PaneRect {
                x: r.left() - lr.left(),
                y: r.top() - lr.top(),
                w: r.width(),
                h: r.height(),
            }
        };
        match anchor_for_place.as_ref() {
            OverlayAnchor::Node => node_el
                .and_then(|n| n.0.get_untracked())
                .map(|el| from_element(el.unchecked_into())),
            OverlayAnchor::Selector(sel) => node_el
                .and_then(|n| n.0.get_untracked())
                .and_then(|el| {
                    el.unchecked_into::<web_sys::Element>()
                        .query_selector(sel)
                        .ok()
                        .flatten()
                })
                .map(from_element),
            OverlayAnchor::Element(get) => get().map(from_element),
            OverlayAnchor::Pane(rect) => Some(PaneRect {
                x: rect.position.x,
                y: rect.position.y,
                w: rect.size.width,
                h: rect.size.height,
            }),
        }
    };

    let place = move || {
        let Some(layer_el) = layer.mount.get_untracked() else {
            return;
        };
        let layer_el: web_sys::Element = layer_el.unchecked_into();
        let Some(a) = resolve_anchor(&layer_el) else {
            return;
        };
        let Some(panel) = panel_ref.get_untracked() else {
            return;
        };

        let lr = layer_el.get_bounding_client_rect();
        let (pane_w, pane_h) = (lr.width(), lr.height());
        let (pw, ph) = (panel.offset_width() as f64, panel.offset_height() as f64);

        let cross = |start: f64, extent: f64, size: f64| match align {
            OverlayAlign::Start => start,
            OverlayAlign::Center => start + (extent - size) / 2.0,
            OverlayAlign::End => start + extent - size,
        };

        let main = |side: OverlaySide| match side {
            OverlaySide::Right => (a.x + a.w + offset, cross(a.y, a.h, ph)),
            OverlaySide::Left => (a.x - offset - pw, cross(a.y, a.h, ph)),
            OverlaySide::Bottom => (cross(a.x, a.w, pw), a.y + a.h + offset),
            OverlaySide::Top => (cross(a.x, a.w, pw), a.y - offset - ph),
        };

        let (mut x, mut y) = main(side);

        if keep_in_view {
            // Flip to the opposite side only if that side actually has room —
            // otherwise the shift below is the better of two bad options.
            let overflows = match side {
                OverlaySide::Right => x + pw > pane_w,
                OverlaySide::Left => x < 0.0,
                OverlaySide::Bottom => y + ph > pane_h,
                OverlaySide::Top => y < 0.0,
            };
            if overflows {
                let opposite = match side {
                    OverlaySide::Right => OverlaySide::Left,
                    OverlaySide::Left => OverlaySide::Right,
                    OverlaySide::Bottom => OverlaySide::Top,
                    OverlaySide::Top => OverlaySide::Bottom,
                };
                let (fx, fy) = main(opposite);
                let fits = match opposite {
                    OverlaySide::Right => fx + pw <= pane_w,
                    OverlaySide::Left => fx >= 0.0,
                    OverlaySide::Bottom => fy + ph <= pane_h,
                    OverlaySide::Top => fy >= 0.0,
                };
                if fits {
                    (x, y) = (fx, fy);
                }
            }
            // Shift into the pane. A panel larger than the pane pins to the
            // top-left rather than being pushed off the other edge.
            x = x.min(pane_w - pw).max(0.0);
            y = y.min(pane_h - ph).max(0.0);
        }

        // Skip no-op writes: this runs every frame while tracking.
        if placement.get_untracked() != Some((x, y)) {
            placement.set(Some((x, y)));
        }
    };

    let place: Arc<dyn Fn() + Send + Sync> = Arc::new(place);

    // Place on mount and whenever the canvas transform changes, so pan and zoom
    // are tracked without depending on animation frames.
    let place_vp = Arc::clone(&place);
    Effect::new(move || {
        let _ = layer.viewport.get();
        let _ = panel_ref.get();
        place_vp();
    });

    // Everything else the panel should follow — node drags, resizes, content
    // reflow — is caught by re-placing each frame.
    if track_anchor {
        let alive = RwSignal::new(true);
        on_cleanup(move || alive.set(false));

        fn tick(f: Arc<dyn Fn() + Send + Sync>, alive: RwSignal<bool>) {
            if !alive.get_untracked() {
                return;
            }
            f();
            crate::raf::request_animation_frame(move || tick(f, alive));
        }
        tick(place, alive);
    }

    // Dismissal: a pointerdown that lands outside the panel, or Escape.
    if let Some(on_dismiss) = on_dismiss {
        let _ = use_event_listener(
            leptos::prelude::document(),
            leptos::ev::pointerdown,
            move |ev: web_sys::PointerEvent| {
                let Some(panel) = panel_ref.get_untracked() else {
                    return;
                };
                let inside = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                    .is_some_and(|n| panel.contains(Some(&n)));
                if !inside {
                    on_dismiss.run(());
                }
            },
        );

        let _ = use_event_listener(
            leptos::prelude::document(),
            leptos::ev::keydown,
            move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    on_dismiss.run(());
                }
            },
        );
    }

    // A Signal (not a closure): the view below can re-run, so this has to be
    // Copy rather than move-once.
    let panel_style = Signal::derive(move || {
        let placed = placement.get();
        let (x, y) = placed.unwrap_or((0.0, 0.0));
        // Hidden until placed, so it never paints in the wrong spot first.
        let vis = if placed.is_some() {
            ""
        } else {
            "visibility: hidden;"
        };
        format!(
            "position: absolute; left: {x}px; top: {y}px; pointer-events: auto; \
             {vis} {extra_style}"
        )
    });

    let backdrop_view = backdrop.then(|| {
        let dismiss = on_dismiss;
        view! {
            <div
                data-node-overlay-backdrop=""
                style="position: absolute; inset: 0; pointer-events: auto;"
                on:pointerdown=move |_| {
                    if let Some(cb) = dismiss {
                        cb.run(());
                    }
                }
            />
        }
    });

    view! {
        {move || {
            layer.mount.get().map(|mount_el| {
                let mount_el: web_sys::Element = mount_el.unchecked_into();
                let children = children.clone();
                let backdrop_view = backdrop_view.clone();
                view! {
                    <Portal mount=mount_el>
                        {backdrop_view.clone()}
                        <div data-node-overlay="" node_ref=panel_ref style=panel_style>
                            {children()}
                        </div>
                    </Portal>
                }
            })
        }}
    }
    .into_any()
}
