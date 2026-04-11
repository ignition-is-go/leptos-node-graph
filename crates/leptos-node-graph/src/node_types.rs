use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use leptos::prelude::*;

use crate::anchor::{InputAnchor, OutputAnchor};
use crate::menu::{NodeMenuItem, TypedNodeDef, TypedPort};
use crate::node::Node;
use crate::types::*;

/// Override for a specific port's slot content.
/// When a port has a PortSlot, its children replace the default label.
type PortSlot = Arc<dyn Fn() -> AnyView + Send + Sync>;

/// Builder for defining a node type.
pub struct NodeTypeBuilder<T: PortType> {
    typed_def: TypedNodeDef<T>,
    /// Optional body content renderer.
    body: Option<Arc<dyn Fn() -> AnyView + Send + Sync>>,
    /// Per-port children overrides (keyed by port id).
    port_slots: HashMap<String, PortSlot>,
    /// Fully custom renderer — if set, skips auto-rendering.
    custom_renderer: Option<Arc<dyn Fn(String, RwSignal<Position>) -> AnyView + Send + Sync>>,
    _marker: PhantomData<T>,
}

impl<T: PortType> NodeTypeBuilder<T> {
    pub fn new(def: TypedNodeDef<T>) -> Self {
        Self {
            typed_def: def,
            body: None,
            port_slots: HashMap::new(),
            custom_renderer: None,
            _marker: PhantomData,
        }
    }

    /// Set the body content (controls, dropdowns, etc. between header and ports).
    pub fn body(mut self, f: impl Fn() -> AnyView + Send + Sync + 'static) -> Self {
        self.body = Some(Arc::new(f));
        self
    }

    /// Override a port's label with custom slot content.
    pub fn port_slot(mut self, port_id: &str, f: impl Fn() -> AnyView + Send + Sync + 'static) -> Self {
        self.port_slots.insert(port_id.to_string(), Arc::new(f));
        self
    }

    /// Use a fully custom renderer instead of auto-generated ports.
    /// Use this for nodes with dynamic port counts.
    pub fn custom_renderer(mut self, f: impl Fn(String, RwSignal<Position>) -> AnyView + Send + Sync + 'static) -> Self {
        self.custom_renderer = Some(Arc::new(f));
        self
    }

    /// Build the node type definition.
    pub fn build(self) -> NodeTypeDef {
        let menu_item = self.typed_def.to_menu_item();

        if let Some(renderer) = self.custom_renderer {
            return NodeTypeDef {
                menu_item,
                renderer,
            };
        }

        let typed_ports = self.typed_def.ports;
        let label = self.typed_def.label;
        let body = self.body;
        let port_slots = self.port_slots;

        let renderer = Arc::new(move |node_id: String, position: RwSignal<Position>| {
            let label = label.clone();
            let typed_ports = typed_ports.clone();
            let body = body.clone();
            let port_slots = port_slots.clone();
            let nid = node_id.clone();

            let inputs: Vec<TypedPort<T>> = typed_ports.iter()
                .filter(|p| p.direction == PortDirection::Input)
                .cloned()
                .collect();
            let outputs: Vec<TypedPort<T>> = typed_ports.iter()
                .filter(|p| p.direction == PortDirection::Output)
                .cloned()
                .collect();

            let nid2 = nid.clone();
            let ps1 = port_slots.clone();
            let inputs_view: Option<Children> = if inputs.is_empty() { None } else {
                Some(Box::new(move || {
                    inputs.iter().map(|port| {
                        let port_id = format!("{}_{}", nid, port.id);
                        let port_type = port.port_type.clone();
                        let marker: PhantomData<(String, String)> = PhantomData;

                        if let Some(slot) = ps1.get(&port.id) {
                            let content = slot();
                            view! {
                                <InputAnchor id=port_id port_type=port_type _marker=marker>
                                    {content}
                                </InputAnchor>
                            }.into_any()
                        } else {
                            let lbl = port.label.clone();
                            view! {
                                <InputAnchor id=port_id port_type=port_type _marker=marker label=lbl />
                            }.into_any()
                        }
                    }).collect_view().into_any()
                }))
            };

            let ps2 = port_slots.clone();
            let outputs_view: Option<Children> = if outputs.is_empty() { None } else {
                Some(Box::new(move || {
                    outputs.iter().map(|port| {
                        let port_id = format!("{}_{}", nid2, port.id);
                        let port_type = port.port_type.clone();
                        let marker: PhantomData<(String, String)> = PhantomData;

                        if let Some(slot) = ps2.get(&port.id) {
                            let content = slot();
                            view! {
                                <OutputAnchor id=port_id port_type=port_type _marker=marker>
                                    {content}
                                </OutputAnchor>
                            }.into_any()
                        } else {
                            let lbl = port.label.clone();
                            view! {
                                <OutputAnchor id=port_id port_type=port_type _marker=marker label=lbl />
                            }.into_any()
                        }
                    }).collect_view().into_any()
                }))
            };

            let header_content: Children = Box::new(move || {
                view! { {label.clone()} }.into_any()
            });

            let body_content: Children = body.map(|b| {
                Box::new(move || b()) as Children
            }).unwrap_or_else(|| Box::new(|| ().into_any()));

            let inputs_final: Children = inputs_view.unwrap_or_else(|| Box::new(|| ().into_any()));
            let outputs_final: Children = outputs_view.unwrap_or_else(|| Box::new(|| ().into_any()));

            let node_marker: PhantomData<(String, String, T)> = PhantomData;

            view! {
                <Node
                    id=node_id
                    position=position
                    _marker=node_marker
                    header=header_content
                    body=body_content
                    inputs=inputs_final
                    outputs=outputs_final
                />
            }.into_any()
        });

        NodeTypeDef {
            menu_item,
            renderer,
        }
    }
}

/// A registered node type: menu info + renderer.
#[derive(Clone)]
pub struct NodeTypeDef {
    /// Menu item info (id, label, category, description, ports).
    pub menu_item: NodeMenuItem,
    /// Renderer function: (node_id, position) -> AnyView.
    renderer: Arc<dyn Fn(String, RwSignal<Position>) -> AnyView + Send + Sync>,
}

impl NodeTypeDef {
    /// Create with a fully custom renderer.
    pub fn custom(
        menu_item: NodeMenuItem,
        renderer: impl Fn(String, RwSignal<Position>) -> AnyView + Send + Sync + 'static,
    ) -> Self {
        Self {
            menu_item,
            renderer: Arc::new(renderer),
        }
    }

    /// Render this node type.
    pub fn render(&self, id: String, position: RwSignal<Position>) -> AnyView {
        (self.renderer)(id, position)
    }
}

/// Registry of all available node types.
#[derive(Clone, Default)]
pub struct NodeTypeRegistry {
    types: HashMap<String, NodeTypeDef>,
    order: Vec<String>,
}

impl NodeTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: NodeTypeDef) {
        let id = def.menu_item.id.clone();
        self.order.push(id.clone());
        self.types.insert(id, def);
    }

    pub fn menu_items(&self) -> Vec<NodeMenuItem> {
        self.order.iter()
            .filter_map(|id| self.types.get(id).map(|d| d.menu_item.clone()))
            .collect()
    }

    pub fn render(&self, type_id: &str, node_id: String, position: RwSignal<Position>) -> Option<AnyView> {
        self.types.get(type_id).map(|def| def.render(node_id, position))
    }

    pub fn get(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.types.get(type_id)
    }
}
