use leptos::prelude::*;
use leptos_use::use_event_listener;

use crate::theme::NodeMenuStyle;
use crate::types::*;

/// A port definition on a menu item.
#[derive(Clone, Debug)]
pub struct MenuPort {
    /// Port identifier (used in the CreateNode event).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Port direction.
    pub direction: PortDirection,
    /// Type identifier for compatibility checking (matched against draft port type).
    pub type_id: String,
}

/// A menu item representing a node type that can be created.
#[derive(Clone, Debug)]
pub struct NodeMenuItem {
    /// Unique identifier for this node type.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Optional category for grouping.
    pub category: Option<String>,
    /// Optional description shown below the label.
    pub description: Option<String>,
    /// Ports this node type has.
    pub ports: Vec<MenuPort>,
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
    let (hovered_port, set_hovered_port) = signal(Option::<(usize, String)>::None);

    // Focus input when menu opens
    Effect::new(move || {
        if open_at.get().is_some() {
            search_text.set(String::new());
            set_selected_index.set(0);
            set_hovered_port.set(None);
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
                    && el.closest("[data-node-menu]").ok().flatten().is_some() {
                        return;
                    }
            }
            open_at.set(None);
            on_event_close.run(NodeMenuEvent::Cancelled);
        },
    );

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
        let item_count = items.with_untracked(|items| items.len());

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
                let item = items.with_untracked(|items| items.get(idx).cloned());
                if let Some(item) = item {
                    let dc = draft_context.get_untracked();
                    let connect_port = dc.and_then(|dc| {
                        let target_dir = match dc.origin_direction {
                            PortDirection::Output => PortDirection::Input,
                            PortDirection::Input => PortDirection::Output,
                        };
                        item.ports
                            .iter()
                            .find(|p| p.direction == target_dir)
                            .map(|p| p.id.clone())
                    });
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

    // Reset selected index when items change
    Effect::new(move || {
        let count = items.with(|items| items.len());
        let current = selected_index.get_untracked();
        if current >= count && count > 0 {
            set_selected_index.set(count - 1);
        }
    });

    move || {
        let _canvas_pos = open_at.get()?;
        let sp = screen_pos.get()?;

        let menu_style = format!(
            "position: fixed; left: {}px; top: {}px; z-index: 10000;",
            sp.x, sp.y,
        );

        let current_items = items.get();
        let selected = selected_index.get();
        let dc = draft_context.get();
        let hp = hovered_port.get();

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
                    <div style="overflow-y: auto; padding: 4px 0;">
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
                            // In draft mode, filter out items with no compatible ports
                            let filtered_items: Vec<_> = if dc.is_some() {
                                current_items.into_iter().filter(|item| {
                                    let dc_ref = dc.as_ref().unwrap();
                                    let target_dir = match dc_ref.origin_direction {
                                        PortDirection::Output => PortDirection::Input,
                                        PortDirection::Input => PortDirection::Output,
                                    };
                                    item.ports.iter().any(|p| {
                                        if p.direction != target_dir { return false; }
                                        let (src, tgt) = if dc_ref.origin_direction == PortDirection::Output {
                                            (dc_ref.source_type_id.clone(), p.type_id.clone())
                                        } else {
                                            (p.type_id.clone(), dc_ref.source_type_id.clone())
                                        };
                                        dc_ref.is_compatible.run((src, tgt))
                                    })
                                }).collect()
                            } else {
                                current_items
                            };

                            let mut last_category: Option<String> = None;
                            filtered_items
                                .into_iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    let is_selected = i == selected;
                                    let dc_inner = dc.clone();
                                    let emit = emit_create;

                                    // Category header
                                    let cat_header = if item.category != last_category {
                                        last_category.clone_from(&item.category);
                                        item.category.clone().map(|cat| {
                                            view! {
                                                <div style=format!(
                                                    "padding: 4px 12px 2px; font-size: 9px; font-weight: 600; \
                                                     text-transform: uppercase; letter-spacing: 0.05em; color: {};",
                                                    ms.category_color
                                                )>
                                                    {cat}
                                                </div>
                                            }
                                        })
                                    } else {
                                        None
                                    };

                                    let desc = item.description.clone();
                                    let item_id = item.id.clone();

                                    // Compatible ports for draft mode
                                    let target_dir = dc_inner.as_ref().map(|dc| {
                                        match dc.origin_direction {
                                            PortDirection::Output => PortDirection::Input,
                                            PortDirection::Input => PortDirection::Output,
                                        }
                                    });

                                    let compatible_ports: Vec<_> = if let (Some(dir), Some(dc)) = (target_dir, &dc_inner) {
                                        item.ports.iter()
                                            .filter(|p| {
                                                if p.direction != dir { return false; }
                                                let (src, tgt) = if dc.origin_direction == PortDirection::Output {
                                                    (dc.source_type_id.clone(), p.type_id.clone())
                                                } else {
                                                    (p.type_id.clone(), dc.source_type_id.clone())
                                                };
                                                dc.is_compatible.run((src, tgt))
                                            })
                                            .cloned()
                                            .collect()
                                    } else {
                                        vec![]
                                    };

                                    let has_visible_ports = compatible_ports.len() > 1;
                                    let item_bg = if is_selected && !has_visible_ports {
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
                                    let has_multi_ports = compatible_ports.len() > 1;

                                    // Port sub-items (always visible inline in draft mode)
                                    let port_views = if compatible_ports.len() > 1 {
                                        let hp = hp.clone();
                                        let ports_html: Vec<_> = compatible_ports.into_iter().map(|port| {
                                            let emit_port = emit;
                                            let iid = item_id.clone();
                                            let pid = port.id.clone();
                                            let dir_icon = match port.direction {
                                                PortDirection::Input => "› ",
                                                PortDirection::Output => "‹ ",
                                            };
                                            let pid_click = pid.clone();
                                            let iid_click = iid.clone();
                                            let pid_hover = pid.clone();
                                            let hover_key = (i, pid.clone());
                                            let is_port_hovered = hp.as_ref() == Some(&hover_key);
                                            let port_bg = if is_port_hovered {
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
                                                    style=port_style
                                                    on:pointerup=move |ev: web_sys::PointerEvent| {
                                                        ev.stop_propagation();
                                                        ev.prevent_default();
                                                        emit_port(iid_click.clone(), Some(pid_click.clone()));
                                                    }
                                                    on:mouseenter=move |_| {
                                                        set_hovered_port.set(Some((i, pid_hover.clone())));
                                                    }
                                                    on:mouseleave=move |_| {
                                                        set_hovered_port.set(None);
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
                                            style=item_style
                                            on:pointerup=move |ev: web_sys::PointerEvent| {
                                                ev.stop_propagation();
                                                if has_multi_ports { return; }
                                                emit_item(item_id_click.clone(), auto_port_click.clone());
                                            }
                                            on:mouseenter=move |_| {
                                                set_selected_index.set(i);
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

fn request_animation_frame(f: impl FnOnce() + 'static) {
    use leptos::wasm_bindgen::prelude::*;
    let cb = Closure::once_into_js(f);
    let _ = web_sys::window()
        .unwrap()
        .request_animation_frame(cb.as_ref().unchecked_ref());
}
