use leptos::prelude::*;
use leptos_use::use_event_listener;

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
#[derive(Clone, Debug)]
pub struct DraftContext {
    pub origin_direction: PortDirection,
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
) -> impl IntoView
{
    provide_context(NodeMenuContext { open_at });

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
    let on_event_close = on_event.clone();
    let _ = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::pointerdown,
        move |ev: web_sys::PointerEvent| {
            if open_at.get_untracked().is_none() {
                return;
            }
            if let Some(target) = ev.target() {
                use leptos::wasm_bindgen::JsCast;
                if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                    if el.closest("[data-node-menu]").ok().flatten().is_some() {
                        return;
                    }
                }
            }
            open_at.set(None);
            on_event_close.run(NodeMenuEvent::Cancelled);
        },
    );

    // Helper: emit create event and close menu
    let emit_create = {
        let on_event = on_event.clone();
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
    let on_event_key = on_event.clone();
    let emit_create_key = emit_create.clone();
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
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
                        item.ports.iter()
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

        Some(view! {
            <div style=menu_style data-node-menu="">
                <div style="background: #1e1e22; border: 1px solid #3f3f46; border-radius: 8px; \
                            box-shadow: 0 8px 24px rgba(0,0,0,0.5); min-width: 220px; \
                            max-height: 360px; display: flex; flex-direction: column; \
                            overflow: hidden;">
                    <div style="padding: 8px; border-bottom: 1px solid #27272a;">
                        <input
                            node_ref=input_ref
                            type="text"
                            placeholder="Search nodes..."
                            style="width: 100%; background: #27272a; border: 1px solid #3f3f46; \
                                   border-radius: 4px; color: #d4d4d8; font-size: 12px; \
                                   padding: 6px 8px; outline: none; box-sizing: border-box;"
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
                                <div style="padding: 12px 16px; color: #71717a; font-size: 12px; \
                                            text-align: center;">
                                    "No matching nodes"
                                </div>
                            }.into_any()
                        } else {
                            let mut last_category: Option<String> = None;
                            current_items
                                .into_iter()
                                .enumerate()
                                .map(|(i, item)| {
                                    let is_selected = i == selected;
                                    let dc_inner = dc.clone();
                                    let emit = emit_create.clone();

                                    // Category header
                                    let cat_header = if item.category != last_category {
                                        last_category.clone_from(&item.category);
                                        item.category.clone().map(|cat| {
                                            view! {
                                                <div style="padding: 4px 12px 2px; font-size: 9px; \
                                                            font-weight: 600; text-transform: uppercase; \
                                                            letter-spacing: 0.05em; color: #52525b;">
                                                    {cat}
                                                </div>
                                            }
                                        })
                                    } else {
                                        None
                                    };

                                    let has_visible_ports = dc_inner.is_some() && item.ports.iter().filter(|p| {
                                        dc_inner.as_ref().map_or(false, |dc| {
                                            let target = match dc.origin_direction {
                                                PortDirection::Output => PortDirection::Input,
                                                PortDirection::Input => PortDirection::Output,
                                            };
                                            p.direction == target
                                        })
                                    }).count() > 1;
                                    let item_bg = if is_selected && !has_visible_ports {
                                        "background: rgba(99, 102, 241, 0.15);"
                                    } else {
                                        ""
                                    };

                                    let item_style = format!(
                                        "padding: 6px 12px; cursor: pointer; font-size: 12px; \
                                         color: #d4d4d8; {item_bg}"
                                    );

                                    let desc = item.description.clone();
                                    let item_id = item.id.clone();

                                    // Compatible ports for draft mode
                                    let target_dir = dc_inner.as_ref().map(|dc| {
                                        match dc.origin_direction {
                                            PortDirection::Output => PortDirection::Input,
                                            PortDirection::Input => PortDirection::Output,
                                        }
                                    });

                                    let compatible_ports: Vec<_> = if let Some(dir) = target_dir {
                                        item.ports.iter()
                                            .filter(|p| p.direction == dir)
                                            .cloned()
                                            .collect()
                                    } else {
                                        vec![]
                                    };

                                    // Auto-connect port for single-port or no-draft cases
                                    let auto_port = if compatible_ports.len() == 1 {
                                        Some(compatible_ports[0].id.clone())
                                    } else {
                                        None
                                    };

                                    // Node item click handler
                                    let emit_item = emit.clone();
                                    let item_id_click = item_id.clone();
                                    let auto_port_click = auto_port.clone();
                                    let has_multi_ports = compatible_ports.len() > 1;

                                    // Port sub-items (always visible inline in draft mode)
                                    let port_views = if compatible_ports.len() > 1 {
                                        let hp = hp.clone();
                                        let ports_html: Vec<_> = compatible_ports.into_iter().map(|port| {
                                            let emit_port = emit.clone();
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
                                                "background: rgba(99, 102, 241, 0.15);"
                                            } else {
                                                ""
                                            };
                                            let port_style = format!(
                                                "padding: 3px 4px 3px 20px; cursor: pointer; \
                                                 font-size: 11px; color: #a1a1aa; {port_bg}"
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
                                                <div style="font-size: 10px; color: #71717a; margin-top: 2px;">
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
