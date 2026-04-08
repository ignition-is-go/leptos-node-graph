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
    // Only handle left click on empty canvas
    if ev.button() != 0 {
        return;
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
    let buttons = ev.buttons();

    // Panning: middle-button (4) or ctrl+left (1 with ctrl)
    if buttons & 4 != 0 || (buttons & 1 != 0 && ev.ctrl_key()) {
        let dx = ev.movement_x() as f64;
        let dy = ev.movement_y() as f64;
        registry.viewport.update(|vp| {
            vp.pan_x += dx;
            vp.pan_y += dy;
        });
        return;
    }

    // Node dragging
    let has_drag = registry.drag_state.with_untracked(|ds| ds.is_some());
    if has_drag {
        let canvas_pos = canvas_pos_from_event(registry, &ev, container_ref);
        let grid_size = registry.config.with_untracked(|c| c.grid_size);

        registry.drag_state.with_untracked(|ds| {
            if let Some(ds) = ds {
                let delta = Position::new(
                    canvas_pos.x - ds.offset.x,
                    canvas_pos.y - ds.offset.y,
                );

                for (node_id, start_pos) in &ds.start_positions {
                    let mut new_pos = Position::new(
                        start_pos.x + delta.x,
                        start_pos.y + delta.y,
                    );
                    if let Some(grid) = grid_size {
                        new_pos = utils::snap_to_grid(new_pos, grid);
                    }
                    registry.set_node_position(node_id, new_pos);
                }
            }
        });
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
        let rect = registry.box_select.with_untracked(|bs| {
            bs.as_ref().map(|bs| bs.to_rect())
        });
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
    // End box select
    registry.box_select.set(None);

    // End node drag - emit NodeMoved events for moved nodes
    let drag = registry.drag_state.with_untracked(|ds| ds.clone());
    if let Some(drag) = drag {
        registry.drag_state.set(None);

        // Emit NodeMoved for each selected node that was dragged
        for (node_id, _start_pos) in &drag.start_positions {
            let current_pos = registry.nodes.with_untracked(|nodes| {
                nodes.get(node_id).map(|n| n.position)
            });
            if let Some(pos) = current_pos {
                registry.emit(GraphEvent::NodeMoved {
                    id: node_id.clone(),
                    position: pos,
                });
            }
        }
    }

    // Cancel draft connection only if mouseup was NOT on an anchor
    // (clicking an anchor to complete a connection is handled by the anchor's mousedown)
    if let Some(target) = ev.target() {
        use leptos::wasm_bindgen::JsCast;
        if let Some(el) = target.dyn_ref::<web_sys::Element>() {
            let is_anchor: bool = el.closest(".anchor").ok().flatten().is_some();
            if !is_anchor {
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

    let zoom_delta = -ev.delta_y() * 0.001;

    registry.viewport.update(|vp| {
        let old_zoom = vp.zoom;
        let new_zoom = (old_zoom + zoom_delta).clamp(min_zoom, max_zoom);
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
        _ => {}
    }
}
