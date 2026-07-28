use leptos::prelude::*;
use leptos_use::use_event_listener;

use crate::theme::NodeMenuStyle;
use crate::types::*;

/// A port definition on a menu item (type-erased for menu display/filtering).
#[derive(Clone, Debug)]
pub struct MenuPort {
    /// Port identifier (used in the CreateNode event).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Port direction.
    pub direction: PortDirection,
    /// Type identifier string for compatibility checking.
    pub type_id: String,
}

/// Category with name and optional color.
#[derive(Clone, Debug, Default)]
pub struct Category {
    pub name: String,
    pub color: Option<String>,
}

impl Category {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
        }
    }

    pub fn with_color(name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: Some(color.into()),
        }
    }
}

/// A menu item representing a node type that can be created (type-erased).
#[derive(Clone, Debug)]
pub struct NodeMenuItem {
    /// Unique identifier for this node type.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Optional category for grouping and header display.
    pub category: Option<Category>,
    /// Optional description shown below the label.
    pub description: Option<String>,
    /// Ports this node type has.
    pub ports: Vec<MenuPort>,
}

/// Typed port definition for use with the builder API.
/// Converts to `MenuPort` automatically.
#[derive(Clone, Debug)]
pub struct TypedPort<T: PortType> {
    pub id: String,
    pub label: String,
    pub direction: PortDirection,
    pub port_type: T,
}

impl<T: PortType> TypedPort<T> {
    pub fn input(id: impl Into<String>, label: impl Into<String>, port_type: T) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            direction: PortDirection::Input,
            port_type,
        }
    }

    pub fn output(id: impl Into<String>, label: impl Into<String>, port_type: T) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            direction: PortDirection::Output,
            port_type,
        }
    }

    /// Convert to a type-erased MenuPort for the menu UI.
    pub fn to_menu_port(&self) -> MenuPort {
        MenuPort {
            id: self.id.clone(),
            label: self.label.clone(),
            direction: self.direction,
            type_id: self.port_type.type_id(),
        }
    }
}

/// Typed node definition for use with the builder API.
/// Converts to `NodeMenuItem` automatically.
pub struct TypedNodeDef<T: PortType> {
    pub id: String,
    pub label: String,
    pub category: Option<Category>,
    pub description: Option<String>,
    pub ports: Vec<TypedPort<T>>,
}

impl<T: PortType> TypedNodeDef<T> {
    /// Convert to a type-erased NodeMenuItem for the menu UI.
    pub fn to_menu_item(&self) -> NodeMenuItem {
        NodeMenuItem {
            id: self.id.clone(),
            label: self.label.clone(),
            category: self.category.clone(),
            description: self.description.clone(),
            ports: self.ports.iter().map(|p| p.to_menu_port()).collect(),
        }
    }
}

/// Events emitted by the node menu.
#[derive(Clone, Debug)]
pub enum NodeMenuEvent {
    /// User selected a node type to create at this canvas position.
    CreateNode {
        item_id: String,
        position: Position,
        connect_to_port: Option<String>,
    },
    /// Menu was closed without selection.
    Cancelled,
}

/// Context for controlling the node menu from outside.
#[derive(Clone)]
pub struct NodeMenuContext {
    pub open_at: RwSignal<Option<Position>>,
}

/// Whether the menu was opened during a draft connection.
#[derive(Clone)]
pub struct DraftContext {
    pub origin_direction: PortDirection,
    /// Type ID of the draft source port (for compatibility filtering).
    pub source_type_id: String,
    /// Compatibility checker: given (output_type_id, input_type_id),
    /// returns true if they can connect.
    pub is_compatible: Callback<(String, String), bool>,
}

impl std::fmt::Debug for DraftContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DraftContext")
            .field("origin_direction", &self.origin_direction)
            .field("source_type_id", &self.source_type_id)
            .finish()
    }
}

/// A searchable node creation menu.
///
/// When opened during a draft connection, compatible ports are shown inline.
#[component]
pub fn NodeMenu(
    /// Reactive list of menu items.
    items: Signal<Vec<NodeMenuItem>>,
    /// Two-way search text signal.
    search_text: RwSignal<String>,
    /// Callback when item is selected or menu is cancelled.
    on_event: Callback<NodeMenuEvent>,
    /// Position to open the menu at (canvas coordinates). None = closed.
    open_at: RwSignal<Option<Position>>,
    /// Screen position for rendering the menu (fixed coordinates).
    screen_pos: Signal<Option<Position>>,
    /// If set, menu was opened during a draft connection.
    #[prop(optional, into)]
    draft_context: Signal<Option<DraftContext>>,
) -> impl IntoView {
    provide_context(NodeMenuContext { open_at });

    let ms = use_context::<NodeMenuStyle>().unwrap_or_default();
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let (selected_index, set_selected_index) = signal(0usize);

    // Focus input when menu opens
    Effect::new(move || {
        if open_at.get().is_some() {
            search_text.set(String::new());
            set_selected_index.set(0);
            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.focus();
                }
            });
        }
    });

    // Close on Escape or click outside (native listener so it works reliably)
    let on_event_close = on_event;
    let _ = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::pointerdown,
        move |ev: web_sys::PointerEvent| {
            if open_at.get_untracked().is_none() {
                return;
            }
            if let Some(target) = ev.target() {
                use leptos::wasm_bindgen::JsCast;
                if let Some(el) = target.dyn_ref::<web_sys::Element>()
                    && el.closest("[data-node-menu]").ok().flatten().is_some()
                {
                    return;
                }
            }
            open_at.set(None);
            on_event_close.run(NodeMenuEvent::Cancelled);
        },
    );

    // The node types on screen: during a draft, only those with a port that
    // could accept the connection.
    //
    // Derived HERE rather than inline in the view because keyboard nav, Enter,
    // and rendering must agree. While the filter lived only in the view,
    // arrowing walked the UNFILTERED count — the highlight ran off the end of
    // the visible rows and stuck there — and Enter resolved that index against
    // the unfiltered list, creating the wrong node.
    let visible_items = Signal::derive(move || {
        let all = items.get();
        let Some(dc) = draft_context.get() else {
            return all;
        };
        all.into_iter()
            .filter(|item| !compatible_ports(item, &dc).is_empty())
            .collect()
    });

    // What the selection actually walks. While a connection is in flight you are
    // choosing a PIN, not a node — so a node offering two compatible ports
    // contributes two entries and the arrows step pin by pin. With no draft (or
    // a single compatible port, where the node row is unambiguous) it stays one
    // entry per node.
    let visible_entries = Signal::derive(move || {
        let all = items.get();
        let Some(dc) = draft_context.get() else {
            return all
                .into_iter()
                .map(|item| (item, None))
                .collect::<Vec<(NodeMenuItem, Option<String>)>>();
        };
        all.into_iter()
            .flat_map(|item| {
                let ports = compatible_ports(&item, &dc);
                match ports.len() {
                    0 => Vec::new(),
                    1 => vec![(item, Some(ports[0].id.clone()))],
                    _ => ports
                        .iter()
                        .map(|p| (item.clone(), Some(p.id.clone())))
                        .collect(),
                }
            })
            .collect()
    });

    // Helper: emit create event and close menu
    let emit_create = {
        move |item_id: String, port_id: Option<String>| {
            if let Some(pos) = open_at.get_untracked() {
                on_event.run(NodeMenuEvent::CreateNode {
                    item_id,
                    position: pos,
                    connect_to_port: port_id,
                });
                open_at.set(None);
            }
        }
    };

    // Keyboard handler
    let on_event_key = on_event;
    let emit_create_key = emit_create;
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        // Stop all keyboard events from reaching the editor
        ev.stop_propagation();
        let item_count = visible_entries.with_untracked(|entries| entries.len());

        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                if item_count > 0 {
                    set_selected_index.update(|i| *i = (*i + 1).min(item_count - 1));
                }
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = i.saturating_sub(1));
            }
            "Enter" => {
                ev.prevent_default();
                let idx = selected_index.get_untracked();
                // The entry already carries the exact pin that was selected —
                // no re-deriving "first compatible port", which would ignore
                // which pin the user actually arrowed to.
                let entry = visible_entries.with_untracked(|entries| entries.get(idx).cloned());
                if let Some((item, connect_port)) = entry {
                    emit_create_key(item.id, connect_port);
                }
            }
            "Escape" | "Tab" => {
                ev.prevent_default();
                open_at.set(None);
                on_event_key.run(NodeMenuEvent::Cancelled);
            }
            _ => {}
        }
    };

    // Keep the highlighted row visible while arrowing through the list. Effects
    // run after the DOM patch, so the direct call is normally enough; the extra
    // frame covers layout that settles later and is a no-op when it doesn't.
    Effect::new(move || {
        let _ = selected_index.get();
        scroll_selected_into_view();
        request_animation_frame(scroll_selected_into_view);
    });

    // Reset selected index when the entry list changes
    Effect::new(move || {
        let count = visible_entries.with(|entries| entries.len());
        let current = selected_index.get_untracked();
        if current >= count && count > 0 {
            set_selected_index.set(count - 1);
        }
    });

    move || {
        let _canvas_pos = open_at.get()?;
        let sp = screen_pos.get()?;

        // Clamp the spawn point to the viewport so the menu never falls off the
        // bottom/right edge (its box is up to min-width 220px × max-height 360px).
        // `max(8)` keeps a small margin off the top/left too.
        let (vw, vh) = web_sys::window()
            .map(|w| {
                (
                    w.inner_width()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1920.0),
                    w.inner_height()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1080.0),
                )
            })
            .unwrap_or((1920.0, 1080.0));
        const MENU_W: f64 = 240.0;
        const MENU_H: f64 = 360.0;
        let left = sp.x.min((vw - MENU_W - 8.0).max(8.0)).max(8.0);
        let top = sp.y.min((vh - MENU_H - 8.0).max(8.0)).max(8.0);

        let menu_style = format!(
            "position: fixed; left: {}px; top: {}px; z-index: 10000;",
            left, top,
        );

        let current_items = visible_items.get();
        let selected = selected_index.get();
        let dc = draft_context.get();

        let panel_style = format!(
            "background: {}; border: {}; border-radius: 8px; box-shadow: {}; \
             min-width: 220px; max-height: 360px; display: flex; flex-direction: column; overflow: hidden;",
            ms.background, ms.border, ms.shadow
        );
        let search_wrapper_style = format!("padding: 8px; border-bottom: {};", ms.divider);
        let input_style = format!(
            "width: 100%; background: {}; border: {}; border-radius: 4px; color: {}; \
             font-size: 12px; padding: 6px 8px; outline: none; box-sizing: border-box;",
            ms.input_background, ms.input_border, ms.input_color
        );

        Some(view! {
            <div style=menu_style data-node-menu="">
                <div style=panel_style>
                    <div style=search_wrapper_style>
                        <input
                            node_ref=input_ref
                            type="text"
                            placeholder="Search nodes..."
                            style=input_style
                            prop:value=move || search_text.get()
                            on:input=move |ev| {
                                use leptos::wasm_bindgen::JsCast;
                                let t = ev.target().unwrap()
                                    .unchecked_into::<web_sys::HtmlInputElement>();
                                search_text.set(t.value());
                                set_selected_index.set(0);
                            }
                            on:keydown=on_keydown
                        />
                    </div>
                    <div data-node-menu-list="" style="overflow-y: auto; padding: 4px 0;">
                        {if current_items.is_empty() {
                            view! {
                                <div style=format!(
                                    "padding: 12px 16px; color: {}; font-size: 12px; text-align: center;",
                                    ms.empty_color
                                )>
                                    "No matching nodes"
                                </div>
                            }.into_any()
                        } else {
                            // Tag each node with the entry index its row (or its
                            // first pin sub-row) corresponds to, so the rendered
                            // highlight lands on exactly what the keyboard has
                            // selected. Must consume entries in the same order
                            // `visible_entries` produces them.
                            let mut rows: Vec<(NodeMenuItem, Vec<MenuPort>, usize)> = Vec::new();
                            let mut next_entry = 0usize;
                            for item in current_items {
                                let ports = dc
                                    .as_ref()
                                    .map(|dc| compatible_ports(&item, dc))
                                    .unwrap_or_default();
                                let consumed = if ports.len() > 1 { ports.len() } else { 1 };
                                rows.push((item, ports, next_entry));
                                next_entry += consumed;
                            }

                            let mut last_category: Option<String> = None; // track by name
                            rows
                                .into_iter()
                                .map(|(item, compatible_ports, base_entry)| {
                                    // A node offering several pins is never itself
                                    // the selection — its pins are.
                                    let has_multi_ports = compatible_ports.len() > 1;
                                    let is_selected = !has_multi_ports && base_entry == selected;
                                    let emit = emit_create;

                                    // Category header
                                    let cat_name = item.category.as_ref().map(|c| c.name.clone());
                                    let cat_header = if cat_name != last_category {
                                        last_category.clone_from(&cat_name);
                                        item.category.clone().map(|cat| {
                                            let color = cat.color.unwrap_or_else(|| ms.category_color.clone());
                                            view! {
                                                <div style=format!(
                                                    "padding: 4px 12px 2px; font-size: 9px; font-weight: 600; \
                                                     text-transform: uppercase; letter-spacing: 0.05em; color: {};",
                                                    color
                                                )>
                                                    {cat.name}
                                                </div>
                                            }
                                        })
                                    } else {
                                        None
                                    };

                                    let desc = item.description.clone();
                                    let item_id = item.id.clone();

                                    let item_bg = if is_selected {
                                        format!("background: {};", ms.hover_background)
                                    } else {
                                        String::new()
                                    };

                                    let item_style = format!(
                                        "padding: 6px 12px; cursor: pointer; font-size: 12px; \
                                         color: {}; {item_bg}", ms.item_color
                                    );

                                    // Auto-connect port for single-port or no-draft cases
                                    let auto_port = if compatible_ports.len() == 1 {
                                        Some(compatible_ports[0].id.clone())
                                    } else {
                                        None
                                    };

                                    // Node item click handler
                                    let emit_item = emit;
                                    let item_id_click = item_id.clone();
                                    let auto_port_click = auto_port.clone();

                                    // Pin sub-rows, each its own selectable entry.
                                    let port_views = if has_multi_ports {
                                        let ports_html: Vec<_> = compatible_ports.into_iter().enumerate().map(|(j, port)| {
                                            let emit_port = emit;
                                            let iid = item_id.clone();
                                            let pid = port.id.clone();
                                            let dir_icon = match port.direction {
                                                PortDirection::Input => "› ",
                                                PortDirection::Output => "‹ ",
                                            };
                                            let pid_click = pid.clone();
                                            let iid_click = iid.clone();
                                            // Keyboard and mouse drive the SAME
                                            // selection, so a pin highlights the
                                            // same way however you reached it.
                                            let entry_idx = base_entry + j;
                                            let is_port_selected = entry_idx == selected;
                                            let port_bg = if is_port_selected {
                                                format!("background: {};", ms.hover_background)
                                            } else {
                                                String::new()
                                            };
                                            let port_style = format!(
                                                "padding: 3px 4px 3px 20px; cursor: pointer; \
                                                 font-size: 11px; color: {}; {port_bg}", ms.port_color
                                            );
                                            view! {
                                                <div
                                                    data-menu-item-selected=is_port_selected.then_some("")
                                                    style=port_style
                                                    on:pointerup=move |ev: web_sys::PointerEvent| {
                                                        ev.stop_propagation();
                                                        ev.prevent_default();
                                                        emit_port(iid_click.clone(), Some(pid_click.clone()));
                                                    }
                                                    on:mouseenter=move |_| {
                                                        set_selected_index.set(entry_idx);
                                                    }
                                                >
                                                    {dir_icon}{port.label}
                                                </div>
                                            }
                                        }).collect();
                                        Some(ports_html.into_iter().collect_view())
                                    } else {
                                        None
                                    };

                                    view! {
                                        {cat_header}
                                        <div
                                            data-menu-item-selected=is_selected.then_some("")
                                            style=item_style
                                            on:pointerup=move |ev: web_sys::PointerEvent| {
                                                ev.stop_propagation();
                                                if has_multi_ports { return; }
                                                emit_item(item_id_click.clone(), auto_port_click.clone());
                                            }
                                            on:mouseenter=move |_| {
                                                if !has_multi_ports {
                                                    set_selected_index.set(base_entry);
                                                }
                                            }
                                        >
                                            {item.label}
                                            {desc.map(|d| view! {
                                                <div style=format!("font-size: 10px; color: {}; margin-top: 2px;", ms.description_color)>
                                                    {d}
                                                </div>
                                            })}
                                            {port_views}
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </div>
            </div>
        })
    }
}

/// The ports on `item` that could accept the in-flight draft connection.
///
/// Single source of truth for "what is compatible": the visible node list, the
/// pin entries the keyboard walks, and the rendered sub-rows all call this, so
/// they cannot drift apart.
fn compatible_ports(item: &NodeMenuItem, dc: &DraftContext) -> Vec<MenuPort> {
    let target_dir = match dc.origin_direction {
        PortDirection::Output => PortDirection::Input,
        PortDirection::Input => PortDirection::Output,
    };
    item.ports
        .iter()
        .filter(|p| {
            if p.direction != target_dir {
                return false;
            }
            let (src, tgt) = if dc.origin_direction == PortDirection::Output {
                (dc.source_type_id.clone(), p.type_id.clone())
            } else {
                (p.type_id.clone(), dc.source_type_id.clone())
            };
            dc.is_compatible.run((src, tgt))
        })
        .cloned()
        .collect()
}

/// Scroll the highlighted row just far enough to be fully visible — the same
/// behavior as `scrollIntoView({block: "nearest"})`, done with client rects so
/// it needs neither a positioned `offsetParent` nor web-sys's
/// `ScrollIntoViewOptions` feature. A row that's already visible doesn't move,
/// which is what keeps mouse hover (which also sets the selection) from
/// scrolling the list under the cursor.
fn scroll_selected_into_view() {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(Some(list)) = doc.query_selector("[data-node-menu] [data-node-menu-list]") else {
        return;
    };
    let Ok(Some(item)) =
        doc.query_selector("[data-node-menu] [data-node-menu-list] [data-menu-item-selected]")
    else {
        return;
    };

    let lr = list.get_bounding_client_rect();
    let ir = item.get_bounding_client_rect();
    let delta = if ir.top() < lr.top() {
        ir.top() - lr.top()
    } else if ir.bottom() > lr.bottom() {
        ir.bottom() - lr.bottom()
    } else {
        return;
    };
    list.set_scroll_top((list.scroll_top() as f64 + delta).round() as i32);
}

fn request_animation_frame(f: impl FnOnce() + 'static) {
    use leptos::wasm_bindgen::prelude::*;
    let cb = Closure::once_into_js(f);
    let _ = web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}
