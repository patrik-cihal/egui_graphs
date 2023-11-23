use egui::Pos2;
use petgraph::stable_graph::DefaultIx;
use petgraph::Directed;

use petgraph::graph::IndexType;
use petgraph::{
    stable_graph::{EdgeIndex, EdgeReference, NodeIndex, StableGraph},
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
    Direction, EdgeType,
};

use crate::draw::{DisplayEdge, DisplayNode};
use crate::{metadata::Metadata, transform, Edge, Node};
use crate::{DefaultEdgeShape, DefaultNodeShape};

type StableGraphType<N, E, Ty, Ix, Dn, De> =
    StableGraph<Node<N, E, Ty, Ix, Dn>, Edge<N, E, Ty, Ix, Dn, De>, Ty, Ix>;

/// Graph type compatible with [`super::GraphView`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Graph<
    N: Clone,
    E: Clone,
    Ty: EdgeType = Directed,
    Ix: IndexType = DefaultIx,
    Dn: DisplayNode<N, E, Ty, Ix> = DefaultNodeShape,
    De: DisplayEdge<N, E, Ty, Ix, Dn> = DefaultEdgeShape,
> {
    pub g: StableGraphType<N, E, Ty, Ix, Dn, De>,
}

impl<N: Clone, E: Clone, Ty: EdgeType, Ix: IndexType, D: DisplayNode<N, E, Ty, Ix>>
    From<&StableGraph<N, E, Ty, Ix>> for Graph<N, E, Ty, Ix, D>
{
    fn from(value: &StableGraph<N, E, Ty, Ix>) -> Self {
        transform::to_graph(value)
    }
}

impl<
        N: Clone,
        E: Clone,
        Ty: EdgeType,
        Ix: IndexType,
        Dn: DisplayNode<N, E, Ty, Ix>,
        De: DisplayEdge<N, E, Ty, Ix, Dn>,
    > Graph<N, E, Ty, Ix, Dn, De>
{
    pub fn new(g: StableGraphType<N, E, Ty, Ix, Dn, De>) -> Self {
        Self { g }
    }

    /// Finds node by position. Can be optimized by using a spatial index like quad-tree if needed.
    pub fn node_by_screen_pos(&self, meta: &Metadata, screen_pos: Pos2) -> Option<NodeIndex<Ix>> {
        let pos_in_graph = meta.screen_to_canvas_pos(screen_pos);
        for (idx, node) in self.nodes_iter() {
            let display = node.display();
            if display.is_inside(pos_in_graph) {
                return Some(idx);
            }
        }
        None
    }

    /// Finds edge by position.
    pub fn edge_by_screen_pos(&self, meta: &Metadata, screen_pos: Pos2) -> Option<EdgeIndex<Ix>> {
        let pos_in_graph = meta.screen_to_canvas_pos(screen_pos);
        for (idx, e) in self.edges_iter() {
            let (idx_start, idx_end) = self.g.edge_endpoints(e.id()).unwrap();
            let start = self.g.node_weight(idx_start).unwrap();
            let end = self.g.node_weight(idx_end).unwrap();
            if e.display().is_inside(start, end, pos_in_graph) {
                return Some(idx);
            }
        }

        None
    }

    pub fn g(&mut self) -> &mut StableGraphType<N, E, Ty, Ix, Dn, De> {
        &mut self.g
    }

    ///Provides iterator over all nodes and their indices.
    pub fn nodes_iter(&self) -> impl Iterator<Item = (NodeIndex<Ix>, &Node<N, E, Ty, Ix, Dn>)> {
        self.g.node_references()
    }

    /// Provides iterator over all edges and their indices.
    pub fn edges_iter(&self) -> impl Iterator<Item = (EdgeIndex<Ix>, &Edge<N, E, Ty, Ix, Dn, De>)> {
        self.g.edge_references().map(|e| (e.id(), e.weight()))
    }

    pub fn node(&self, i: NodeIndex<Ix>) -> Option<&Node<N, E, Ty, Ix, Dn>> {
        self.g.node_weight(i)
    }

    pub fn edge(&self, i: EdgeIndex<Ix>) -> Option<&Edge<N, E, Ty, Ix, Dn, De>> {
        self.g.edge_weight(i)
    }

    pub fn edge_endpoints(&self, i: EdgeIndex<Ix>) -> Option<(NodeIndex<Ix>, NodeIndex<Ix>)> {
        self.g.edge_endpoints(i)
    }

    pub fn node_mut(&mut self, i: NodeIndex<Ix>) -> Option<&mut Node<N, E, Ty, Ix, Dn>> {
        self.g.node_weight_mut(i)
    }

    pub fn edge_mut(&mut self, i: EdgeIndex<Ix>) -> Option<&mut Edge<N, E, Ty, Ix, Dn, De>> {
        self.g.edge_weight_mut(i)
    }

    pub fn is_directed(&self) -> bool {
        self.g.is_directed()
    }

    pub fn edges_num(&self, idx: NodeIndex<Ix>) -> usize {
        self.g.edges(idx).count()
    }

    pub fn edges_directed(
        &self,
        idx: NodeIndex<Ix>,
        dir: Direction,
    ) -> impl Iterator<Item = EdgeReference<Edge<N, E, Ty, Ix, Dn, De>, Ix>> {
        self.g.edges_directed(idx, dir)
    }
}
