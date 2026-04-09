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
    /// Ports this node type has. Shown as sub-items when menu is opened during a draft connection.
    pub ports: Vec<MenuPort>,
}

/// Events emitted by the node menu.
#[derive(Clone, Debug)]
pub enum NodeMenuEvent {
    /// User selected a node type to create at this canvas position.
    /// If `connect_to_port` is Some, the consumer should also create a connection
    /// between the draft source and this port on the new node.
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
    /// Set to Some(position) to open the menu at that canvas position. None to close.
    pub open_at: RwSignal<Option<Position>>,
}

/// Whether the menu was opened during a draft connection, and from which direction.
#[derive(Clone, Debug)]
pub struct DraftContext {
    /// The direction of the port that started the draft.
    /// The menu will show ports of the OPPOSITE direction as connectable sub-items.
    pub origin_direction: PortDirection,
}

/// A searchable node creation menu.
///
/// When opened during a draft connection, compatible ports are shown as sub-items.
/// Selecting a port creates the node AND completes the connection in one action.
#[component]
pub fn NodeMenu(
    /// Reactive list of menu items. Consumer filters these based on search_text.
    items: Signal<Vec<NodeMenuItem>>,
    /// Two-way search text signal. Menu writes to it as user types.
    search_text: RwSignal<String>,
    /// Callback when item is selected or menu is cancelled.
    on_event: Callback<NodeMenuEvent>,
    /// Canvas position to open the menu at. None = closed.
    open_at: RwSignal<Option<Position>>,
    /// Current viewport transform for canvas-to-screen conversion.
    viewport: Signal<ViewportTransform>,
    /// If set, the menu was opened during a draft connection.
    /// Ports of the opposite direction will be shown as connectable sub-items.
    #[prop(optional, into)]
    draft_context: Signal<Option<DraftContext>>,
) -> impl IntoView
{
    provide_context(NodeMenuContext { open_at });

    let input_ref = NodeRef::<leptos::html::Input>::new();
    let (selected_index, set_selected_index) = signal(0usize);
    // When a node item is expanded, show its ports. None = no expansion.
    let (expanded_item, set_expanded_item) = signal(Option::<usize>::None);

    // Reset state when menu opens
    Effect::new(move || {
        if open_at.get().is_some() {
            search_text.set(String::new());
            set_selected_index.set(0);
            set_expanded_item.set(None);
            request_animation_frame(move || {
                if let Some(el) = input_ref.get() {
                    let _ = el.focus();
                }
            });
        }
    });

    // Close on click outside
    let on_event_close = on_event.clone();
    let _ = use_event_listener(
        leptos::prelude::document(),
        leptos::ev::mousedown,
        move |ev: web_sys::MouseEvent| {
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

    // Keyboard handler
    let on_event_key = on_event.clone();
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
            "ArrowRight" => {
                // Expand to show ports if in draft context
                let dc = draft_context.get_untracked();
                if dc.is_some() {
                    let idx = selected_index.get_untracked();
                    set_expanded_item.set(Some(idx));
                }
            }
            "ArrowLeft" => {
                set_expanded_item.set(None);
            }
            "Enter" => {
                ev.prevent_default();
                let idx = selected_index.get_untracked();
                let item = items.with_untracked(|items| items.get(idx).cloned());
                if let Some(item) = item {
                    if let Some(pos) = open_at.get_untracked() {
                        let dc = draft_context.get_untracked();
                        let connect_port = if dc.is_some() {
                            // Auto-select first compatible port
                            let target_dir = match dc.unwrap().origin_direction {
                                PortDirection::Output => PortDirection::Input,
                                PortDirection::Input => PortDirection::Output,
                            };
                            item.ports.iter()
                                .find(|p| p.direction == target_dir)
                                .map(|p| p.id.clone())
                        } else {
                            None
                        };

                        on_event_key.run(NodeMenuEvent::CreateNode {
                            item_id: item.id,
                            position: pos,
                            connect_to_port: connect_port,
                        });
                        open_at.set(None);
                    }
                }
            }
            "Escape" => {
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
        set_expanded_item.set(None);
    });

    move || {
        let pos = open_at.get()?;

        let vp = viewport.get();
        let screen_pos = vp.canvas_to_screen(pos);

        let menu_style = format!(
            "position: absolute; left: {}px; top: {}px; z-index: 10000;",
            screen_pos.x, screen_pos.y,
        );

        let current_items = items.get();
        let selected = selected_index.get();
        let expanded = expanded_item.get();
        let dc = draft_context.get();

        let on_event_click = on_event.clone();

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
                                set_expanded_item.set(None);
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
                                    let is_expanded = expanded == Some(i);
                                    let on_ev = on_event_click.clone();
                                    let dc_inner = dc.clone();

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

                                    let item_bg = if is_selected {
                                        "background: rgba(99, 102, 241, 0.15);"
                                    } else {
                                        ""
                                    };

                                    let has_ports = dc_inner.is_some() && !item.ports.is_empty();
                                    let arrow = if has_ports { " ›" } else { "" };

                                    let item_style = format!(
                                        "padding: 6px 12px; cursor: pointer; font-size: 12px; \
                                         color: #d4d4d8; display: flex; justify-content: space-between; \
                                         align-items: center; {item_bg}"
                                    );

                                    let desc = item.description.clone();
                                    let item_id = item.id.clone();
                                    let item_ports = item.ports.clone();

                                    // Determine which ports to show as sub-items
                                    let target_dir = dc_inner.as_ref().map(|dc| {
                                        match dc.origin_direction {
                                            PortDirection::Output => PortDirection::Input,
                                            PortDirection::Input => PortDirection::Output,
                                        }
                                    });

                                    let compatible_ports: Vec<_> = if let Some(dir) = target_dir {
                                        item_ports.iter()
                                            .filter(|p| p.direction == dir)
                                            .cloned()
                                            .collect()
                                    } else {
                                        vec![]
                                    };

                                    let on_ev_item = on_ev.clone();
                                    let item_id_click = item_id.clone();

                                    view! {
                                        {cat_header}
                                        <div
                                            style=item_style
                                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                                ev.stop_propagation();
                                                if compatible_ports.is_empty() {
                                                    // No ports to pick — just create the node
                                                    if let Some(pos) = open_at.get_untracked() {
                                                        on_ev_item.run(NodeMenuEvent::CreateNode {
                                                            item_id: item_id_click.clone(),
                                                            position: pos,
                                                            connect_to_port: None,
                                                        });
                                                        open_at.set(None);
                                                    }
                                                } else {
                                                    // Expand to show port sub-items
                                                    set_expanded_item.set(Some(i));
                                                }
                                            }
                                            on:mouseenter=move |_| {
                                                set_selected_index.set(i);
                                            }
                                        >
                                            <div>
                                                {item.label}
                                                {desc.map(|d| view! {
                                                    <div style="font-size: 10px; color: #71717a; margin-top: 2px;">
                                                        {d}
                                                    </div>
                                                })}
                                            </div>
                                            <span style="color: #52525b; font-size: 14px;">{arrow}</span>
                                        </div>
                                        // Port sub-items when expanded
                                        {if is_expanded && !item_ports.is_empty() {
                                            let target_dir2 = target_dir;
                                            let ports: Vec<_> = item_ports.iter()
                                                .filter(|p| target_dir2.map_or(true, |d| p.direction == d))
                                                .cloned()
                                                .collect();

                                            if ports.is_empty() {
                                                None
                                            } else {
                                                let on_ev_port = on_ev.clone();
                                                let item_id_port = item_id.clone();
                                                Some(ports.into_iter().map(move |port| {
                                                    let on_ev_p = on_ev_port.clone();
                                                    let iid = item_id_port.clone();
                                                    let pid = port.id.clone();
                                                    let dir_icon = match port.direction {
                                                        PortDirection::Input => "› ",
                                                        PortDirection::Output => "‹ ",
                                                    };
                                                    view! {
                                                        <div
                                                            style="padding: 4px 12px 4px 28px; cursor: pointer; \
                                                                   font-size: 11px; color: #a1a1aa;"
                                                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                                                ev.stop_propagation();
                                                                if let Some(pos) = open_at.get_untracked() {
                                                                    on_ev_p.run(NodeMenuEvent::CreateNode {
                                                                        item_id: iid.clone(),
                                                                        position: pos,
                                                                        connect_to_port: Some(pid.clone()),
                                                                    });
                                                                    open_at.set(None);
                                                                }
                                                            }
                                                        >
                                                            {dir_icon}{port.label}
                                                        </div>
                                                    }
                                                }).collect_view())
                                            }
                                        } else {
                                            None
                                        }}
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
