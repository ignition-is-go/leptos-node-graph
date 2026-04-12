use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use leptos::prelude::*;

use crate::anchor::{InputAnchor, OutputAnchor};
use crate::menu::{Category, NodeMenuItem, TypedNodeDef, TypedPort};
use crate::node::Node;
use crate::types::*;

/// Override for a specific port's slot content. Takes the port label.
type PortSlot = Arc<dyn Fn(String) -> AnyView + Send + Sync>;

/// Render a single port as an InputAnchor or OutputAnchor.
fn render_port<T: PortType>(
    node_id: &str,
    port: &TypedPort<T>,
    port_slots: &HashMap<String, PortSlot>,
    global_slots: &PortTypeSlots,
    is_input: bool,
) -> AnyView {
    let port_id = format!("{}_{}", node_id, port.id);
    let port_type = port.port_type.clone();
    let marker: PhantomData<(String, String)> = PhantomData;

    // Resolve slot: per-port > global type > default label
    let slot_content = port_slots.get(&port.id)
        .map(|s| s(port.label.clone()))
        .or_else(|| {
            let key = (port.port_type.type_id(), port.direction);
            global_slots.get(&key).map(|s| s(port.label.clone()))
        });

    if is_input {
        if let Some(content) = slot_content {
            view! { <InputAnchor id=port_id port_type=port_type _marker=marker>{content}</InputAnchor> }.into_any()
        } else {
            let lbl = port.label.clone();
            view! { <InputAnchor id=port_id port_type=port_type _marker=marker label=lbl /> }.into_any()
        }
    } else {
        if let Some(content) = slot_content {
            view! { <OutputAnchor id=port_id port_type=port_type _marker=marker>{content}</OutputAnchor> }.into_any()
        } else {
            let lbl = port.label.clone();
            view! { <OutputAnchor id=port_id port_type=port_type _marker=marker label=lbl /> }.into_any()
        }
    }
}

/// Builder for defining a node type.
/// Function that produces a reactive port list for a specific node instance.
type DynamicPortsFn<T> = Arc<dyn Fn(String) -> Signal<Vec<TypedPort<T>>> + Send + Sync>;

/// Renderer function signature: (node_id, position) -> AnyView.
type CustomRendererFn = Arc<dyn Fn(String, RwSignal<Position>) -> AnyView + Send + Sync>;

/// Internal renderer: (node_id, position, global_port_type_slots, header_renderer) -> AnyView.
type RendererFn = Arc<dyn Fn(String, RwSignal<Position>, &PortTypeSlots, &Option<HeaderRenderer>) -> AnyView + Send + Sync>;

pub struct NodeTypeBuilder<T: PortType> {
    typed_def: TypedNodeDef<T>,
    /// Optional body content renderer. Receives node_id.
    body: Option<Arc<dyn Fn(String) -> AnyView + Send + Sync>>,
    /// Per-port children overrides (keyed by port id).
    port_slots: HashMap<String, PortSlot>,
    /// Dynamic input ports — called per node instance with node_id, returns reactive port list.
    dynamic_inputs: Option<DynamicPortsFn<T>>,
    /// Dynamic output ports — called per node instance with node_id, returns reactive port list.
    dynamic_outputs: Option<DynamicPortsFn<T>>,
    /// Fully custom renderer — if set, skips auto-rendering.
    custom_renderer: Option<CustomRendererFn>,
    _marker: PhantomData<T>,
}

impl<T: PortType> NodeTypeBuilder<T> {
    pub fn new(def: TypedNodeDef<T>) -> Self {
        Self {
            typed_def: def,
            body: None,
            port_slots: HashMap::new(),
            dynamic_inputs: None,
            dynamic_outputs: None,
            custom_renderer: None,
            _marker: PhantomData,
        }
    }

    /// Set the body content. Receives node_id for per-instance customization.
    pub fn body(mut self, f: impl Fn(String) -> AnyView + Send + Sync + 'static) -> Self {
        self.body = Some(Arc::new(f));
        self
    }

    /// Set dynamic input ports — a function called per node instance that returns
    /// a reactive signal of ports. Ports are re-rendered when the signal changes.
    /// Static ports from the TypedNodeDef are rendered first, dynamic ones after.
    pub fn dynamic_inputs(mut self, f: impl Fn(String) -> Signal<Vec<TypedPort<T>>> + Send + Sync + 'static) -> Self {
        self.dynamic_inputs = Some(Arc::new(f));
        self
    }

    /// Set dynamic output ports — same as dynamic_inputs but for outputs.
    pub fn dynamic_outputs(mut self, f: impl Fn(String) -> Signal<Vec<TypedPort<T>>> + Send + Sync + 'static) -> Self {
        self.dynamic_outputs = Some(Arc::new(f));
        self
    }

    /// Override a specific port's content. The function receives the port label.
    pub fn port_slot(mut self, port_id: &str, f: impl Fn(String) -> AnyView + Send + Sync + 'static) -> Self {
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
                renderer: Arc::new(move |id, pos, _, _| renderer(id, pos)),
            };
        }

        let typed_ports = self.typed_def.ports;
        let label = self.typed_def.label;
        let category = self.typed_def.category;
        let body = self.body;
        let port_slots = self.port_slots;
        let dynamic_inputs = self.dynamic_inputs;
        let dynamic_outputs = self.dynamic_outputs;

        let renderer = Arc::new(move |node_id: String, position: RwSignal<Position>, global_slots: &PortTypeSlots, header_renderer: &Option<HeaderRenderer>| {
            let label = label.clone();
            let typed_ports = typed_ports.clone();
            let body = body.clone();
            let port_slots = port_slots.clone();
            let global_slots = global_slots.clone();
            let dynamic_inputs = dynamic_inputs.clone();
            let dynamic_outputs = dynamic_outputs.clone();
            let nid = node_id.clone();

            let static_inputs: Vec<TypedPort<T>> = typed_ports.iter()
                .filter(|p| p.direction == PortDirection::Input)
                .cloned()
                .collect();
            let static_outputs: Vec<TypedPort<T>> = typed_ports.iter()
                .filter(|p| p.direction == PortDirection::Output)
                .cloned()
                .collect();

            // Build inputs ViewFn: static ports + optional dynamic ports
            let nid_in = nid.clone();
            let ps1 = port_slots.clone();
            let gs1 = global_slots.clone();
            let dyn_in = dynamic_inputs.as_ref().map(|f| f(nid.clone()));
            let inputs_fn = ViewFn::from(move || {
                let mut views: Vec<AnyView> = static_inputs.iter()
                    .map(|port| render_port::<T>(&nid_in, port, &ps1, &gs1, true))
                    .collect();

                if let Some(ref dyn_signal) = dyn_in {
                    for port in &dyn_signal.get() {
                        views.push(render_port::<T>(&nid_in, port, &ps1, &gs1, true));
                    }
                }

                views.into_iter().collect_view().into_any()
            });

            let nid_out = nid.clone();
            let ps2 = port_slots.clone();
            let gs2 = global_slots.clone();
            let dyn_out = dynamic_outputs.as_ref().map(|f| f(nid.clone()));
            let outputs_fn = ViewFn::from(move || {
                let mut views: Vec<AnyView> = static_outputs.iter()
                    .map(|port| render_port::<T>(&nid_out, port, &ps2, &gs2, false))
                    .collect();

                if let Some(ref dyn_signal) = dyn_out {
                    for port in &dyn_signal.get() {
                        views.push(render_port::<T>(&nid_out, port, &ps2, &gs2, false));
                    }
                }

                views.into_iter().collect_view().into_any()
            });

            let category = category.clone();
            let accent = category.as_ref().and_then(|c| c.color.clone()).unwrap_or_default();
            let custom_header = header_renderer.clone();
            let header_content: Children = Box::new(move || {
                if let Some(ref renderer) = custom_header {
                    return renderer(label.clone(), category.clone());
                }
                // Default header: label left, colored category right
                let cat_view = category.as_ref().map(|cat| {
                    let color = cat.color.clone().unwrap_or_else(|| "#71717a".into());
                    let name = cat.name.clone();
                    let style = format!(
                        "color: {color}; font-weight: 400; margin-left: auto; font-size: 10px;"
                    );
                    view! { <span style=style>{name}</span> }
                });
                view! {
                    <div style="display: flex; justify-content: space-between; align-items: center; width: 100%;">
                        <span>{label.clone()}</span>
                        {cat_view}
                    </div>
                }.into_any()
            });

            let nid_body = node_id.clone();
            let body_content: Children = body.map(|b| {
                Box::new(move || b(nid_body.clone())) as Children
            }).unwrap_or_else(|| Box::new(|| ().into_any()));

            let node_marker: PhantomData<(String, String, T)> = PhantomData;

            view! {
                <Node
                    id=node_id
                    position=position
                    _marker=node_marker
                    header=header_content
                    body=body_content
                    inputs=inputs_fn
                    outputs=outputs_fn
                    accent_color=accent
                />
            }.into_any()
        });

        NodeTypeDef {
            menu_item,
            renderer,
        }
    }
}

/// Global port type slots map.
pub type PortTypeSlots = HashMap<(String, PortDirection), PortSlot>;

/// Header renderer: (label, category) -> AnyView.
pub type HeaderRenderer = Arc<dyn Fn(String, Option<Category>) -> AnyView + Send + Sync>;

/// A registered node type: menu info + renderer.
#[derive(Clone)]
pub struct NodeTypeDef {
    /// Menu item info (id, label, category, description, ports).
    pub menu_item: NodeMenuItem,
    /// Renderer function: (node_id, position, global_port_type_slots) -> AnyView.
    renderer: RendererFn,
}

impl NodeTypeDef {
    /// Create with a fully custom renderer (ignores global port type slots).
    pub fn custom(
        menu_item: NodeMenuItem,
        renderer: impl Fn(String, RwSignal<Position>) -> AnyView + Send + Sync + 'static,
    ) -> Self {
        Self {
            menu_item,
            renderer: Arc::new(move |id, pos, _, _| renderer(id, pos)),
        }
    }

    /// Render this node type.
    pub fn render(&self, id: String, position: RwSignal<Position>, global_slots: &PortTypeSlots, header_renderer: &Option<HeaderRenderer>) -> AnyView {
        (self.renderer)(id, position, global_slots, header_renderer)
    }
}

/// Registry of all available node types.
#[derive(Clone, Default)]
pub struct NodeTypeRegistry {
    types: HashMap<String, NodeTypeDef>,
    order: Vec<String>,
    /// Global port type slot overrides.
    port_type_slots: HashMap<(String, PortDirection), PortSlot>,
    /// Global header renderer. If set, used for all auto-rendered nodes.
    header_renderer: Option<HeaderRenderer>,
}

impl NodeTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a global header renderer for all auto-rendered nodes.
    /// Receives (label, category) and returns header content.
    pub fn set_header_renderer(
        &mut self,
        f: impl Fn(String, Option<Category>) -> AnyView + Send + Sync + 'static,
    ) {
        self.header_renderer = Some(Arc::new(f));
    }

    /// Register a global slot renderer for all ports of a given type and direction.
    /// This applies to ALL nodes unless a specific port_slot override is set.
    pub fn register_port_type_slot<T: PortType>(
        &mut self,
        port_type: T,
        direction: PortDirection,
        f: impl Fn(String) -> AnyView + Send + Sync + 'static,
    ) {
        self.port_type_slots.insert(
            (port_type.type_id(), direction),
            Arc::new(f),
        );
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
        self.types.get(type_id).map(|def| def.render(node_id, position, &self.port_type_slots, &self.header_renderer))
    }

    pub fn get(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.types.get(type_id)
    }
}
