use std::collections::HashMap;
use std::marker::PhantomData;

use leptos::prelude::*;
use leptos_use::{UseElementSizeReturn, use_element_size, use_event_listener};

use crate::registry::{DragState, EditorRegistry};
// AnchorStyle is read by anchor.rs, not directly by node.rs
use crate::types::*;

/// Context provided by a Node to its children.
#[derive(Clone)]
pub struct NodeContext<N: NodeId> {
    pub id: N,
    pub position: RwSignal<Position>,
    pub is_selected: Signal<bool>,
    pub is_dragging: Signal<bool>,
    /// Measured height of the header + body sections above the ports.
    /// Used by anchors for deterministic position calculation.
    pub ports_y_offset: Signal<f64>,
    /// Measured node width. Used by output anchors for X position.
    pub node_width: Signal<f64>,
}

/// Per-node visibility flag: whether the node's rectangle intersects the visible
/// viewport (plus an overscan margin). Provided (non-generically) in each node's
/// context so consumers can gate expensive per-anchor content — e.g. live value
/// subscriptions — on it. Off-screen nodes stay MOUNTED (their ports remain
/// registered, so connection wires are unaffected); only their costly content is
/// skipped.
#[derive(Clone, Copy)]
pub struct NodeVisible(pub Signal<bool>);

#[component]
pub fn Node<N, P, C, T>(
    id: N,
    position: RwSignal<Position>,
    #[prop(optional)] _marker: PhantomData<(P, C, T)>,
    /// Header content (title bar). Library measures its height.
    #[prop(optional)] header: Option<Children>,
    /// Body content (dropdowns, controls, etc.) between header and ports.
    /// Library measures its height.
    #[prop(optional)] body: Option<Children>,
    /// Input port anchors (left column). Use ViewFn for reactive/dynamic ports.
    #[prop(optional, into)] inputs: ViewFn,
    /// Output port anchors (right column). Use ViewFn for reactive/dynamic ports.
    #[prop(optional, into)] outputs: ViewFn,
    /// Per-node accent color (the top stripe). Reactive: accepts a `String`, a
    /// `Signal<String>`, or a closure. `None`/empty = no accent bar. Reactive so
    /// async-resolved colors (e.g. a node's category/service color) update the
    /// stripe once they load.
    #[prop(optional, into)] accent_color: MaybeProp<String>,
    /// Per-node header background override. When `Some`, overrides
    /// `NodeStyle.header_background` for this node only (e.g. category colors).
    #[prop(optional, into)] header_color: Option<String>,
) -> impl IntoView
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let registry = expect_context::<EditorRegistry<N, P, C, T>>();
    let node_ref = NodeRef::<leptos::html::Div>::new();
    let header_ref = NodeRef::<leptos::html::Div>::new();
    let body_ref = NodeRef::<leptos::html::Div>::new();

    // Measure header + body heights
    let UseElementSizeReturn { width: node_w, height: node_h } = use_element_size(node_ref);
    let ports_ref = NodeRef::<leptos::html::Div>::new();

    // Measure ports div offset from node top — exact, no padding math needed.
    // Re-derives when node height changes (triggered by header/body resize).
    let ports_y_offset = Signal::derive(move || {
        let _nh = node_h.get(); // track node size changes to re-derive
        ports_ref.with(|el| {
            el.as_ref().map(|e| e.offset_top() as f64).unwrap_or(0.0)
        })
    });
    let node_width = Signal::derive(move || node_w.get());

    // Viewport visibility: does this node's rect intersect the visible canvas
    // area (screen 0..container mapped to canvas space), padded by an overscan
    // margin so nodes near the edge stay live and don't pop while panning?
    // Until the container is measured (size 0), default to visible so nothing is
    // ever wrongly culled during load.
    // MUST be a Memo (value-deduped), NOT Signal::derive: the viewport changes
    // every pan frame, so a non-deduped derived signal would re-fire — and thus
    // rebuild every visible node's content + subscriptions — on EVERY frame. The
    // Memo only notifies when the bool actually flips (a node crossing the edge),
    // so panning is smooth. A generous overscan keeps edge nodes live so they
    // don't pop/rebuild during a normal pan.
    let reg_vis = registry.clone();
    let in_viewport = Memo::new(move |_| {
        let cs = reg_vis.container_size.get();
        if cs.width <= 0.0 || cs.height <= 0.0 {
            return true;
        }
        // Read the DEBOUNCED viewport, not the live one, so visibility (and the
        // subscription create/dispose it drives) only re-evaluates after a pan
        // settles — never mid-pan.
        let vp = reg_vis.visibility_viewport.get();
        let zoom = if vp.zoom.abs() < 1e-6 { 1.0 } else { vp.zoom };
        let tl = vp.screen_to_canvas(Position::new(0.0, 0.0));
        let br = vp.screen_to_canvas(Position::new(cs.width, cs.height));
        let margin = 600.0 / zoom;
        let (vx0, vy0) = (tl.x - margin, tl.y - margin);
        let (vx1, vy1) = (br.x + margin, br.y + margin);
        let p = position.get();
        let nw = node_w.get().max(1.0);
        let nh = node_h.get().max(1.0);
        p.x <= vx1 && p.x + nw >= vx0 && p.y <= vy1 && p.y + nh >= vy0
    });

    // Derived state signals
    let id_sel = id.clone();
    let reg_sel = registry.clone();
    let is_selected =
        Signal::derive(move || reg_sel.selected_nodes.with(|sel| sel.contains(&id_sel)));

    let id_drag = id.clone();
    let reg_drag = registry.clone();
    let is_dragging = Signal::derive(move || {
        reg_drag
            .drag_state
            .with(|ds| ds.as_ref().is_some_and(|d| d.node_id == id_drag))
    });

    // Provide context with measurements
    let ctx = NodeContext {
        id: id.clone(),
        position,
        is_selected,
        is_dragging,
        ports_y_offset,
        node_width,
    };
    provide_context(ctx);
    provide_context(NodeVisible(in_viewport.into()));

    // Register node
    let initial_pos = position.get_untracked();
    registry.register_node(id.clone(), initial_pos, Some(position));

    // Deregister on cleanup
    let reg_cleanup = registry.clone();
    let id_cleanup = id.clone();
    on_cleanup(move || {
        reg_cleanup.deregister_node(&id_cleanup);
    });

    // Sync position signal -> registry (skip during drag to avoid cycles)
    let reg_pos = registry.clone();
    let id_pos = id.clone();
    Effect::new(move || {
        let pos = position.get();
        let dragging = reg_pos.drag_state.with_untracked(|ds| ds.is_some());
        if !dragging {
            reg_pos.set_node_position(&id_pos, pos);
        }
    });

    // Track node size
    let reg_size = registry.clone();
    let size_id = id.clone();
    Effect::new(move || {
        let w = node_w.get();
        let h = node_h.get();
        if w > 0.0 || h > 0.0 {
            let size = Size::new(w, h);
            reg_size.set_node_size(&size_id, size);
            reg_size.emit(GraphEvent::NodeResized {
                id: size_id.clone(),
                size,
            });
        }
    });

    // Mouse down handler
    let reg_md = registry.clone();
    let id_md = id.clone();
    let _ = use_event_listener(
        node_ref,
        leptos::ev::mousedown,
        move |ev: web_sys::MouseEvent| {
            if ev.button() != 0 {
                return;
            }

            // Skip anchor dots and form elements
            if let Some(target) = ev.target() {
                use leptos::wasm_bindgen::JsCast;
                if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                    if el.closest("[data-anchor-dot]").ok().flatten().is_some() {
                        return;
                    }
                    let tag = el.tag_name().to_uppercase();
                    if matches!(
                        tag.as_str(),
                        "INPUT" | "SELECT" | "TEXTAREA" | "BUTTON" | "OPTION"
                    ) {
                        return;
                    }
                }
            }

            ev.stop_propagation();

            let node_id = id_md.clone();

            if ev.shift_key() {
                reg_md.toggle_node_selection(node_id);
            } else {
                let already_selected = reg_md
                    .selected_nodes
                    .with_untracked(|sel| sel.contains(&node_id));
                if !already_selected {
                    reg_md.select_node(node_id.clone());
                }

                let viewport = reg_md.viewport.get_untracked();
                let container_rect = node_ref.with_untracked(|el| {
                    el.as_ref().and_then(|el| {
                        el.closest(".node-editor")
                            .ok()
                            .flatten()
                            .map(|container| container.get_bounding_client_rect())
                    })
                });

                let (offset_x, offset_y) = container_rect
                    .map(|r| (r.left(), r.top()))
                    .unwrap_or((0.0, 0.0));

                let canvas_mouse = viewport.screen_to_canvas(Position::new(
                    ev.client_x() as f64 - offset_x,
                    ev.client_y() as f64 - offset_y,
                ));

                let start_positions: HashMap<N, Position> =
                    reg_md.selected_nodes.with_untracked(|sel| {
                        reg_md.nodes.with_untracked(|nodes| {
                            sel.iter()
                                .filter_map(|nid| {
                                    nodes.get(nid).map(|entry| (nid.clone(), entry.position))
                                })
                                .collect()
                        })
                    });

                reg_md.drag_state.set(Some(DragState {
                    node_id: node_id.clone(),
                    offset: canvas_mouse,
                    start_positions,
                    alt_key: ev.alt_key(),
                }));
            }
        },
    );

    let ns = use_context::<crate::theme::NodeStyle>().unwrap_or_default();
    let ns2 = ns.clone();
    let node_style = move || {
        let pos = position.get();
        let selected = is_selected.get();
        let dragging = is_dragging.get();

        let outline = if selected { format!("outline: {};", ns.outline_selected) } else { String::new() };
        let shadow = if selected { &ns.shadow_selected } else { &ns.shadow };
        let opacity = if dragging { ns.opacity_dragging } else { 1.0 };

        // Fixed width when the theme sets one (body content elides via the
        // node's `overflow: hidden`); otherwise size to content above min-width.
        let width = if ns.width.is_empty() {
            String::new()
        } else {
            format!("width: {};", ns.width)
        };

        format!(
            "position: absolute; left: {}px; top: {}px; \
             background: {}; border: {}; border-radius: {}; \
             min-width: {}; {width} box-shadow: {shadow}; user-select: none; \
             opacity: {opacity}; overflow: hidden; {outline}",
            pos.x, pos.y, ns.background, ns.border, ns.border_radius, ns.min_width,
        )
    };

    // Render header and body slots
    let header_view = header.map(|h| h());
    // ViewFn is Fn (not FnOnce) — wrapping in move || makes it reactive

    let px = ns2.padding_x;
    let header_bg = header_color.unwrap_or_else(|| ns2.header_background.clone());
    let header_style = format!(
        "padding: {}px {}px; border-bottom: {}; background: {}; \
         font-size: {}; font-weight: 600; letter-spacing: 0.03em; \
         text-transform: uppercase; color: {};",
        ns2.header_padding_y, px, ns2.header_border_bottom,
        header_bg,
        ns2.header_font_size, ns2.header_color,
    );
    let has_body = body.is_some();
    let body_view = body.map(|b| b());
    let body_style = if has_body {
        format!(
            "padding: {}px {}px; border-bottom: {};",
            ns2.body_padding_y, px, ns2.body_border_bottom,
        )
    } else {
        String::new()
    };
    // In Stacked layout the ports section is a single vertical column (inputs
    // then outputs below); in Columns it's two side-by-side columns.
    let stacked = ns2.anchor_layout == crate::theme::AnchorLayout::Stacked;
    let ports_style = if stacked {
        format!(
            "display: flex; flex-direction: column; padding: {}px 0;",
            ns2.ports_padding_y,
        )
    } else {
        format!(
            "display: flex; justify-content: space-between; padding: {}px 0;",
            ns2.ports_padding_y,
        )
    };
    // Stacked rows span the full node width (dot side comes from row-reverse on
    // outputs); Columns hugs outputs to the right edge of their column.
    let inputs_style = "display: flex; flex-direction: column;";
    let outputs_style = if stacked {
        "display: flex; flex-direction: column;"
    } else {
        "display: flex; flex-direction: column; align-items: flex-end;"
    };

    let accent_h = ns2.header_accent_height;
    let accent_view = move || {
        accent_color.get().filter(|c| !c.is_empty()).map(|c| {
            let accent_style =
                format!("height: {accent_h}px; background: {c}; flex-shrink: 0;");
            view! { <div style=accent_style /> }
        })
    };

    view! {
        <div
            style=node_style
            node_ref=node_ref
            data-node=""
        >
            {accent_view}
            <div node_ref=header_ref data-node-header="" style=header_style>
                {header_view}
            </div>
            <div node_ref=body_ref data-node-body="" style=body_style>
                {body_view}
            </div>
            <div node_ref=ports_ref data-node-ports="" style=ports_style>
                <div data-node-inputs="" style=inputs_style>
                    {move || inputs.run()}
                </div>
                <div data-node-outputs="" style=outputs_style>
                    {move || outputs.run()}
                </div>
            </div>
        </div>
    }
}

/// A labeled field row for use inside the node body slot.
/// Renders label + children in a flex row, styled from `NodeStyle`.
#[component]
pub fn NodeField(#[prop(into)] label: String, children: Children) -> impl IntoView {
    let ns = use_context::<crate::theme::NodeStyle>().unwrap_or_default();
    let label_style = format!(
        "font-size: {}; color: {}; text-transform: uppercase; \
         letter-spacing: 0.04em; min-width: {};",
        ns.field_label_font_size, ns.field_label_color, ns.field_label_min_width,
    );
    let row_style = format!(
        "display: flex; align-items: center; gap: {};",
        ns.field_gap,
    );
    view! {
        <div style=row_style>
            <label style=label_style>{label}</label>
            {children()}
        </div>
    }
}
