use std::marker::PhantomData;

use petgraph::{
    stable_graph::{DefaultIx, EdgeIndex, IndexType},
    EdgeType,
};

use crate::{DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode};

/// Stores properties of an [Edge]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct EdgeProps<E: Clone> {
    pub payload: E,
    pub order: usize,
    pub selected: bool,
}

/// Stores properties of an edge that can be changed. Used to apply changes to the graph.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Edge<
    N: Clone,
    E: Clone,
    Ty: EdgeType,
    Ix: IndexType = DefaultIx,
    Dn: DisplayNode<N, E, Ty, Ix> = DefaultNodeShape,
    D: DisplayEdge<N, E, Ty, Ix, Dn> = DefaultEdgeShape,
> {
    id: Option<EdgeIndex<Ix>>,

    display: D,

    props: EdgeProps<E>,
    _marker: PhantomData<(N, Ty, Dn)>,
}

impl<
        N: Clone,
        E: Clone,
        Ty: EdgeType,
        Ix: IndexType,
        Dn: DisplayNode<N, E, Ty, Ix>,
        D: DisplayEdge<N, E, Ty, Ix, Dn>,
    > Edge<N, E, Ty, Ix, Dn, D>
{
    pub fn new(payload: E) -> Self {
        let props = EdgeProps {
            payload,

            order: usize::default(),
            selected: bool::default(),
        };

        let display = D::from(props.clone());
        Self {
            props,
            display,

            id: Default::default(),
            _marker: Default::default(),
        }
    }

    pub fn props(&self) -> &EdgeProps<E> {
        &self.props
    }

    /// Binds edge to the actual node ends and fixes its index in the set of duplicate edges.
    pub fn bind(&mut self, idx: EdgeIndex<Ix>, order: usize) {
        self.id = Some(idx);
        self.props.order = order;
    }

    pub fn display(&self) -> &D {
        &self.display
    }

    pub fn display_mut(&mut self) -> &mut D {
        &mut self.display
    }

    pub fn id(&self) -> EdgeIndex<Ix> {
        self.id.unwrap()
    }

    pub fn order(&self) -> usize {
        self.props.order
    }

    pub fn set_order(&mut self, order: usize) {
        self.props.order = order;
    }

    pub fn payload(&self) -> &E {
        &self.props.payload
    }

    pub fn payload_mut(&mut self) -> &mut E {
        &mut self.props.payload
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.props.selected = selected;
    }

    pub fn selected(&self) -> bool {
        self.props.selected
    }
}
