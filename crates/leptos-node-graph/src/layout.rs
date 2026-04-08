use std::collections::HashMap;

use crate::types::*;

#[derive(Clone, Debug)]
pub struct LayoutGraph<N: NodeId, P: PortId> {
    pub nodes: HashMap<N, Size>,
    pub edges: Vec<(N, N)>,
    pub port_to_node: HashMap<P, N>,
}

pub trait LayoutEngine<N: NodeId, P: PortId> {
    fn compute(&self, graph: &LayoutGraph<N, P>) -> HashMap<N, Position>;
}
