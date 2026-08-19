use leptos::prelude::*;
use web_sys::{KeyboardEvent, MouseEvent, WheelEvent};

use crate::registry::{BoxSelect, EditorRegistry};
use crate::types::*;
use crate::utils;

/// Get canvas-space position from a mouse event relative to a container element.
fn canvas_pos_from_event<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: &MouseEvent,
    container_ref: &NodeRef<leptos::html::Div>,
) -> Position
where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    let viewport = registry.viewport.get_untracked();
    let (offset_x, offset_y) = container_ref.with_untracked(|el| {
        if let Some(el) = el {
            let rect = el.get_bounding_client_rect();
            (rect.left(), rect.top())
        } else {
            (0.0, 0.0)
        }
    });
    let screen = Position::new(
        ev.client_x() as f64 - offset_x,
        ev.client_y() as f64 - offset_y,
    );
    viewport.screen_to_canvas(screen)
}

pub fn handle_canvas_mousedown<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: MouseEvent,
    container_ref: &NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    // Pan gesture start: middle button, or ctrl+left. This handler is on THIS
    // container, so the flag scopes the pan to the pressed graph — the global
    // mousemove listener checks it, so sibling graphs don't pan too. Cleared on
    // mouseup.
    if ev.button() == 1 || (ev.button() == 0 && ev.ctrl_key()) {
        registry.is_panning.set(true);
        registry.pan_origin.set(Some(Position::new(
            ev.client_x() as f64,
            ev.client_y() as f64,
        )));
        return;
    }

    // Only handle left click
    if ev.button() != 0 {
        return;
    }

    // Skip if click was on a node or anchor (those have their own handlers)
    if let Some(target) = ev.target() {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let on_node: bool = el.closest("[data-node]").ok().flatten().is_some();
            let on_anchor: bool = el.closest("[data-anchor]").ok().flatten().is_some();
            if on_node || on_anchor {
                return;
            }
        }
    }

    // Cancel any draft connection
    registry.draft_connection.set(None);

    // Clear selection unless shift is held
    if !ev.shift_key() {
        registry.clear_selection();
    }

    // Start box select. Alt changes the same gesture into group creation on
    // mouseup, which keeps the familiar selection rectangle as the preview.
    let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
    registry.box_select.set(Some(BoxSelect {
        start: canvas_pos,
        current: canvas_pos,
        create_group: ev.alt_key(),
    }));
}

pub fn handle_canvas_mousemove<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: MouseEvent,
    container_ref: &NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    // Node width resize. Written straight to the node's width signal rather than
    // RAF-batched like a drag: one node, one style recompute per move, and the
    // measured-size effect it triggers is already ResizeObserver-throttled.
    let resize = registry.resize_state.with_untracked(|rs| rs.clone());
    if let Some(rs) = resize {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
        let mut width = rs.start_width + (canvas_pos.x - rs.start_x);
        width = width.max(rs.min_width);
        if let Some(max) = rs.max_width {
            width = width.min(max);
        }
        rs.width_signal.set(Some(width));
        return;
    }

    // Panning — ONLY if this instance started the pan (see mousedown). The move
    // listener is document-level, so this per-instance gate is what stops a drag
    // from panning every open graph at once.
    if registry.is_panning.get_untracked() {
        // Delta measured from the last frame's CLIENT position — see
        // `pan_origin`. `movementX/Y` would be in physical pixels here, which
        // pans faster than the cursor on any display where the device pixel
        // ratio isn't 1. Both are identical at ratio 1, so this costs nothing
        // where the old path already behaved.
        let now = Position::new(ev.client_x() as f64, ev.client_y() as f64);
        let last = registry.pan_origin.get_untracked().unwrap_or(now);
        registry.pan_origin.set(Some(now));
        let (dx, dy) = (now.x - last.x, now.y - last.y);
        registry.viewport.update(|vp| {
            vp.pan_x += dx;
            vp.pan_y += dy;
        });
        return;
    }

    // Node dragging — store latest position and schedule RAF
    let has_drag = registry.drag_state.with_untracked(|ds| ds.is_some());
    if has_drag {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
        registry.pending_drag_pos.set(Some(canvas_pos));

        if !registry.drag_raf_pending.get_untracked() {
            registry.drag_raf_pending.set(true);
            let reg = registry.clone();
            crate::raf::request_animation_frame(move || {
                reg.drag_raf_pending.set(false);
                let canvas_pos = reg.pending_drag_pos.get_untracked();
                if let Some(canvas_pos) = canvas_pos {
                    reg.pending_drag_pos.set(None);
                    let grid_size = reg.config.with_untracked(|c| c.grid_size);
                    reg.drag_state.with_untracked(|ds| {
                        if let Some(ds) = ds {
                            let delta = Position::new(
                                canvas_pos.x - ds.offset.x,
                                canvas_pos.y - ds.offset.y,
                            );
                            let updates: Vec<(N, Position)> = ds
                                .start_positions
                                .iter()
                                .map(|(node_id, start_pos)| {
                                    let mut new_pos =
                                        Position::new(start_pos.x + delta.x, start_pos.y + delta.y);
                                    if let Some(grid) = grid_size {
                                        new_pos = utils::snap_to_grid(new_pos, grid);
                                    }
                                    (node_id.clone(), new_pos)
                                })
                                .collect();
                            reg.batch_set_positions(&updates);
                        }
                    });
                }
            });
        }
        return;
    }

    // Box select update
    let has_box = registry.box_select.with_untracked(|bs| bs.is_some());
    if has_box {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
        registry.box_select.update(|bs| {
            if let Some(bs) = bs {
                bs.current = canvas_pos;
            }
        });

        // Select nodes in rect
        let rect = registry
            .box_select
            .with_untracked(|bs| bs.as_ref().map(|bs| bs.to_rect()));
        if let Some(rect) = rect {
            let nodes_in = registry.nodes_in_rect(&rect);
            registry.selected_nodes.set(nodes_in);
        }
        return;
    }

    // Draft connection: update end position, pulling to a nearby compatible port.
    let has_draft = registry.draft_connection.with_untracked(|dc| dc.is_some());
    if has_draft {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);

        // The radius is configured in screen pixels and divided by zoom, so the
        // pull feels identical whether you're zoomed in or out.
        let snap_distance = registry.config.with_untracked(|c| c.snap_distance);
        let snap = if snap_distance > 0.0 {
            let zoom = registry.viewport.get_untracked().zoom;
            let zoom = if zoom.abs() < 1e-6 { 1.0 } else { zoom };
            registry.snap_target_for_draft(canvas_pos, snap_distance / zoom)
        } else {
            None
        };

        registry.draft_connection.update(|dc| {
            if let Some(dc) = dc {
                match &snap {
                    Some((port_id, port_pos)) => {
                        dc.current_end = *port_pos;
                        dc.snap_target = Some(port_id.clone());
                    }
                    None => {
                        dc.current_end = canvas_pos;
                        dc.snap_target = None;
                    }
                }
            }
        });
    }
}

/// Handle a document-level mouseup. Ends box-select and node-drag (emitting
/// `NodesMoved`), then resolves an in-flight draft connection.
///
/// The draft is cancelled UNLESS the mouseup landed on an anchor or the node
/// menu: a successful connection is completed by the target anchor's own
/// handler, and an anchor mouseup that *didn't* complete (the initial click of a
/// click-to-start/click-to-complete connection) must keep the draft alive so the
/// follow-up click can finish it. Dropping on empty canvas cancels the draft —
/// the creation menu is opened with Tab during a draft, not by dropping.
pub fn handle_canvas_mouseup<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: MouseEvent,
    _container_ref: &NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    // End pan (this instance) and box select.
    let was_panning = registry.is_panning.get_untracked();
    registry.is_panning.set(false);
    if was_panning {
        registry.pan_origin.set(None);
    }
    let box_select = registry.box_select.get_untracked();
    registry.box_select.set(None);

    if box_select.is_some_and(|bs| bs.create_group) {
        let ids: Vec<N> = registry
            .selected_nodes
            .with_untracked(|selected| selected.iter().cloned().collect());
        if ids.len() > 1 {
            registry.emit(GraphEvent::GroupCreated { node_ids: ids });
        }
    }

    // End a width resize. No event to emit here — the node's measured-size effect
    // has been emitting `NodeResized` throughout the gesture.
    if registry.resize_state.with_untracked(|rs| rs.is_some()) {
        registry.resize_state.set(None);
    }

    // End node drag - emit one event containing every moved node. Consumers
    // commonly persist graph edits as commands, so preserving the gesture
    // boundary lets them apply a multi-selection move atomically.
    let drag = registry.drag_state.with_untracked(|ds| ds.clone());
    if let Some(drag) = drag {
        registry.drag_state.set(None);

        let nodes: Vec<_> = registry.nodes.with_untracked(|current| {
            drag.start_positions
                .keys()
                .filter_map(|node_id| {
                    current
                        .get(node_id)
                        .map(|node| (node_id.clone(), node.position))
                })
                .collect()
        });
        if !nodes.is_empty() {
            registry.emit(GraphEvent::NodesMoved { nodes });
        }
    }

    // Drafting is a LEFT-button gesture, so only a left release can end it.
    // Releasing a pan (middle-drag, or ctrl+left-drag) must leave an in-flight
    // connection alone — otherwise middle-clicking to pan mid-wiring silently
    // drops the draft. Panning doesn't move the draft endpoint either: the pan
    // shifts the canvas 1:1 with the cursor, so the canvas point under the
    // pointer — which is what the endpoint stores — doesn't change.
    if was_panning || ev.button() != 0 {
        return;
    }

    // The node-creation menu owns the draft while it is open: it derives its
    // compatible-port sub-items from it, and wires the created node to it. Tab
    // is pressed mid-drag, so the release that follows is just the user letting
    // go of the button they started the draft with — it must not end the
    // gesture, or the menu loses its port list AND the node is created unwired.
    // Both menu exits clear the draft themselves (`NodeMenuEvent::CreateNode`
    // and `::Cancelled`), so nothing leaks by deferring to them.
    if registry.menu_open.get_untracked() {
        return;
    }

    // Released while snapped to a port: connect to it. The wire was already
    // drawn attached to that port, so anything else would be a lie — and this is
    // what makes the snap radius, rather than the dot itself, the drop target.
    // (An anchor the pointer is literally over completes via its own mouseup,
    // which runs first and clears the draft, so this can't double-fire.)
    let snapped = registry
        .draft_connection
        .with_untracked(|dc| dc.as_ref().and_then(|d| d.snap_target.clone()));
    if let Some(port_id) = snapped {
        let direction = registry.get_port(&port_id).map(|p| p.direction);
        if let Some(direction) = direction
            && crate::anchor::try_complete_connection(registry, &port_id, direction)
        {
            return;
        }
    }

    // Cancel draft connection only if mouseup was NOT on an anchor or the node menu.
    if let Some(target) = ev.target() {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let is_anchor = el.closest("[data-anchor]").ok().flatten().is_some();
            let is_menu = el.closest("[data-node-menu]").ok().flatten().is_some();
            if !is_anchor && !is_menu {
                registry.draft_connection.set(None);
            }
        } else {
            registry.draft_connection.set(None);
        }
    } else {
        registry.draft_connection.set(None);
    }
}

/// Regions the wheel always scrolls rather than zooming: the node creation menu,
/// `NodeOverlay` panels (and their backdrop), and anything a consumer opts out
/// with `data-graph-no-zoom` — the explicit override for content that should
/// swallow the wheel even when it isn't (yet) overflowing.
const NO_ZOOM_SELECTOR: &str =
    "[data-node-menu],[data-node-overlay],[data-node-overlay-backdrop],[data-graph-no-zoom]";

/// Whether this wheel belongs to scrollable content rather than to the canvas.
///
/// Anything overflowing under the pointer wins the wheel — a node body's list, a
/// popup panel, an inspector — so consumers get correct behavior without tagging
/// every scroller. Position within the scroller is deliberately NOT considered:
/// hitting the end of a list mid-flick should stop, not start zooming.
fn wheel_scrolls_content(ev: &WheelEvent, container: &web_sys::Element) -> bool {
    use leptos::wasm_bindgen::JsCast;
    let Some(target) = ev
        .target()
        .and_then(|t| t.dyn_ref::<web_sys::Element>().cloned())
    else {
        return false;
    };

    if target.closest(NO_ZOOM_SELECTOR).ok().flatten().is_some() {
        return true;
    }

    // Walk target → container looking for an overflowing scroll box.
    let win = leptos::prelude::window();
    let mut node = Some(target);
    while let Some(el) = node {
        if el.is_same_node(Some(container)) {
            return false;
        }
        let overflows = el.scroll_height() > el.client_height();
        if overflows
            && win
                .get_computed_style(&el)
                .ok()
                .flatten()
                .and_then(|s| s.get_property_value("overflow-y").ok())
                .is_some_and(|o| o == "auto" || o == "scroll")
        {
            return true;
        }
        node = el.parent_element();
    }
    // Target outside the container (a portalled panel): treat it as content.
    true
}

pub fn handle_wheel<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: WheelEvent,
    container_ref: &NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    // A wheel over scrollable content belongs to that content, not to the canvas:
    // zooming `prevent_default`s, so without this bail a popup list, node body or
    // inspector can never be scrolled — the graph zooms instead. The overlay
    // backdrop counts too: an open panel is placed once in pane space, so zooming
    // behind it would leave it stranded away from its trigger.
    let scrolls_content = container_ref.with_untracked(|el| {
        el.as_ref()
            .is_some_and(|el| wheel_scrolls_content(&ev, AsRef::<web_sys::Element>::as_ref(el)))
    });
    if scrolls_content {
        return;
    }

    ev.prevent_default();

    let (min_zoom, max_zoom) = registry.config.with_untracked(|c| (c.min_zoom, c.max_zoom));

    let (offset_x, offset_y) = container_ref.with_untracked(|el| {
        if let Some(el) = el {
            let rect = el.get_bounding_client_rect();
            (rect.left(), rect.top())
        } else {
            (0.0, 0.0)
        }
    });

    let mouse_x = ev.client_x() as f64 - offset_x;
    let mouse_y = ev.client_y() as f64 - offset_y;

    // Geometric zoom by wheel DIRECTION only (not deltaY magnitude), matching
    // `@panzoom/panzoom`'s `zoomWithWheel`: `scale * exp((isIn ? 1 : -1) * step)`
    // with the default `step = 0.3` (~+35% in / −26% out per notch). Using the
    // sign rather than the magnitude keeps the per-notch feel identical across
    // mice/trackpads and `deltaMode`s (a magnitude-based factor zoomed at wildly
    // different speeds depending on the device's deltaY).
    const ZOOM_STEP: f64 = 0.3;
    let dir = if ev.delta_y() < 0.0 { 1.0 } else { -1.0 };
    let zoom_factor = (dir * ZOOM_STEP).exp();

    registry.viewport.update(|vp| {
        let old_zoom = vp.zoom;
        let new_zoom = (old_zoom * zoom_factor).clamp(min_zoom, max_zoom);
        let scale_change = new_zoom / old_zoom;

        // Zoom toward mouse position
        vp.pan_x = mouse_x - (mouse_x - vp.pan_x) * scale_change;
        vp.pan_y = mouse_y - (mouse_y - vp.pan_y) * scale_change;
        vp.zoom = new_zoom;
    });
}

pub fn handle_keydown<N, P, C, T>(
    registry: &EditorRegistry<N, P, C, T>,
    ev: KeyboardEvent,
    _container_ref: &NodeRef<leptos::html::Div>,
) where
    N: NodeId,
    P: PortId,
    C: ConnectionId,
    T: PortType,
{
    // Never hijack keys while the user is typing in a node's own controls: a
    // bare "f" would re-frame the graph mid-word, and Delete would delete the
    // selected nodes out from under them.
    if let Some(target) = ev.target() {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let tag = el.tag_name().to_uppercase();
            let editable = matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || el.closest("[contenteditable]").ok().flatten().is_some();
            if editable {
                return;
            }
        }
    }

    let key = ev.key();
    let ctrl = ev.ctrl_key() || ev.meta_key();
    let shift = ev.shift_key();

    match key.as_str() {
        "Delete" | "Backspace" => {
            ev.prevent_default();
            registry.delete_selected();
        }
        "a" if ctrl => {
            ev.prevent_default();
            registry.select_all();
        }
        // Frame the whole graph.
        "f" | "F" if !ctrl => {
            ev.prevent_default();
            registry.fit_view();
        }
        "c" if ctrl => {
            ev.prevent_default();
            let ids: Vec<N> = registry
                .selected_nodes
                .with_untracked(|sel| sel.iter().cloned().collect());
            registry.emit(GraphEvent::NodesCopied { ids });
        }
        "v" if ctrl => {
            ev.prevent_default();
            registry.emit(GraphEvent::NodesPasted {
                offset: Position::new(20.0, 20.0),
            });
        }
        "z" if ctrl && shift => {
            ev.prevent_default();
            registry.emit(GraphEvent::Redo);
        }
        "z" if ctrl => {
            ev.prevent_default();
            registry.emit(GraphEvent::Undo);
        }
        "g" if ctrl => {
            ev.prevent_default();
            let ids: Vec<N> = registry
                .selected_nodes
                .with_untracked(|sel| sel.iter().cloned().collect());
            registry.emit(GraphEvent::GroupCreated { node_ids: ids });
        }
        "Escape" => {
            // Abort an in-flight resize back to the width it started at.
            let resize = registry.resize_state.with_untracked(|rs| rs.clone());
            if let Some(rs) = resize {
                rs.width_signal.set(Some(rs.start_width));
                registry.resize_state.set(None);
            }
            registry.draft_connection.set(None);
            registry.clear_selection();
        }
        // Tab is handled by the editor directly (opens menu)
        _ => {}
    }
}
