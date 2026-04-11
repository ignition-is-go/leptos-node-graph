use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;

use crate::menu::NodeMenuItem;
use crate::types::*;

/// A registered node type: menu info + renderer.
#[derive(Clone)]
pub struct NodeTypeDef {
    /// Menu item info (id, label, category, description, ports).
    pub menu_item: NodeMenuItem,
    /// Renderer function: (node_id, position) -> AnyView.
    renderer: Arc<dyn Fn(String, RwSignal<Position>) -> AnyView + Send + Sync>,
}

impl NodeTypeDef {
    /// Create a new node type definition.
    pub fn new(
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
/// Provides menu items and rendering by type ID.
#[derive(Clone, Default)]
pub struct NodeTypeRegistry {
    types: HashMap<String, NodeTypeDef>,
    /// Ordered type IDs (preserves registration order for menu display).
    order: Vec<String>,
}

impl NodeTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node type.
    pub fn register(&mut self, def: NodeTypeDef) {
        let id = def.menu_item.id.clone();
        self.order.push(id.clone());
        self.types.insert(id, def);
    }

    /// Get the menu items in registration order.
    pub fn menu_items(&self) -> Vec<NodeMenuItem> {
        self.order.iter()
            .filter_map(|id| self.types.get(id).map(|d| d.menu_item.clone()))
            .collect()
    }

    /// Render a node by its type ID.
    pub fn render(&self, type_id: &str, node_id: String, position: RwSignal<Position>) -> Option<AnyView> {
        self.types.get(type_id).map(|def| def.render(node_id, position))
    }

    /// Get a type definition by ID.
    pub fn get(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.types.get(type_id)
    }
}
