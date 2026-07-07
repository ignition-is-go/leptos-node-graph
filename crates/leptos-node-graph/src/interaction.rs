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

    // Start box select
    let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
    registry.box_select.set(Some(BoxSelect {
        start: canvas_pos,
        current: canvas_pos,
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
    // Panning — ONLY if this instance started the pan (see mousedown). The move
    // listener is document-level, so this per-instance gate is what stops a drag
    // from panning every open graph at once.
    if registry.is_panning.get_untracked() {
        let dx = ev.movement_x() as f64;
        let dy = ev.movement_y() as f64;
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

    // Draft connection: update end position
    let has_draft = registry.draft_connection.with_untracked(|dc| dc.is_some());
    if has_draft {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
        registry.draft_connection.update(|dc| {
            if let Some(dc) = dc {
                dc.current_end = canvas_pos;
            }
        });
    }
}

/// Handle a document-level mouseup. Ends box-select and node-drag (emitting
/// `NodeMoved`), then resolves an in-flight draft connection.
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
    registry.is_panning.set(false);
    registry.box_select.set(None);

    // End node drag - emit NodeMoved events for moved nodes
    let drag = registry.drag_state.with_untracked(|ds| ds.clone());
    if let Some(drag) = drag {
        registry.drag_state.set(None);

        // Emit NodeMoved for each selected node that was dragged
        for node_id in drag.start_positions.keys() {
            let current_pos = registry
                .nodes
                .with_untracked(|nodes| nodes.get(node_id).map(|n| n.position));
            if let Some(pos) = current_pos {
                registry.emit(GraphEvent::NodeMoved {
                    id: node_id.clone(),
                    position: pos,
                });
            }
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
            registry.draft_connection.set(None);
            registry.clear_selection();
        }
        // Tab is handled by the editor directly (opens menu)
        _ => {}
    }
}
