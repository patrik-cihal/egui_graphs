use crate::{Edge, Graph, Node};
use egui::Vec2;
use petgraph::{
    stable_graph::{EdgeIndex, NodeIndex, StableGraph},
    visit::IntoNodeReferences,
    EdgeType,
};
use rand::Rng;
use std::collections::HashMap;

pub const DEFAULT_SPAWN_SIZE: f32 = 250.;

/// Helper function which adds user's node to the [`super::Graph`] instance.
///
/// If graph is not empty it picks any node position and adds new node in the vicinity of it.
pub fn add_node<N: Clone, E: Clone, Ty: EdgeType>(g: &mut Graph<N, E, Ty>, n: &N) -> NodeIndex {
    add_node_custom(g, n, default_node_transform)
}

/// Helper function which adds user's node to the [`super::Graph`] instance with custom node transform function.
///
/// If graph is not empty it picks any node position and adds new node in the vicinity of it.
pub fn add_node_custom<N: Clone, E: Clone, Ty: EdgeType>(
    g: &mut Graph<N, E, Ty>,
    n: &N,
    node_transform: impl Fn(NodeIndex, &N) -> Node<N>,
) -> NodeIndex {
    g.g.add_node(node_transform(NodeIndex::new(g.g.node_count() + 1), n))
}

/// Helper function which adds user's edge to the [`super::Graph`] instance.
pub fn add_edge<N: Clone, E: Clone, Ty: EdgeType>(
    g: &mut Graph<N, E, Ty>,
    start: NodeIndex,
    end: NodeIndex,
    e: &E,
) -> EdgeIndex {
    add_edge_custom(g, start, end, e, default_edge_transform)
}

/// Helper function which adds user's edge to the [`super::Graph`] instance with custom edge transform function.
pub fn add_edge_custom<N: Clone, E: Clone, Ty: EdgeType>(
    g: &mut Graph<N, E, Ty>,
    start: NodeIndex,
    end: NodeIndex,
    e: &E,
    edge_transform: impl Fn(EdgeIndex, &E) -> Edge<E>,
) -> EdgeIndex {
    g.g.add_edge(
        start,
        end,
        edge_transform(EdgeIndex::new(g.g.edge_count() + 1), e),
    )
}

/// Helper function which transforms users [`petgraph::stable_graph::StableGraph`] isntance into the version required by the [`super::GraphView`] widget.
///
/// The function creates a new StableGraph where the nodes and edges are encapsulated into
/// Node and Edge structs respectively. New nodes and edges are created with [`default_node_transform`] and [`default_edge_transform`]
/// functions. If you want to define custom transformation procedures (e.g. to use custom label for nodes), use [`to_graph_custom`] instead.
///
/// # Arguments
/// * `g` - A reference to a [`petgraph::stable_graph::StableGraph`]. The graph can have any data type for nodes and edges, and
/// can be either directed or undirected.
///
/// # Returns
/// * A new [`petgraph::stable_graph::StableGraph`] with the same topology as the input graph, but the nodes and edges encapsulated
/// into Node and Edge structs compatible as an input to [`super::GraphView`] widget.
///
/// # Example
/// ```
/// use petgraph::stable_graph::StableGraph;
/// use egui_graphs::to_graph;
/// use egui::Vec2;
///
/// let mut user_graph: StableGraph<&str, &str> = StableGraph::new();
/// let node1 = user_graph.add_node("A");
/// let node2 = user_graph.add_node("B");
/// user_graph.add_edge(node1, node2, "edge1");
///
/// let input_graph = to_graph(&user_graph);
///
/// assert_eq!(input_graph.g.node_count(), 2);
/// assert_eq!(input_graph.g.edge_count(), 1);
///
/// let mut input_indices = input_graph.g.node_indices();
/// let input_node_1 = input_indices.next().unwrap();
/// let input_node_2 = input_indices.next().unwrap();
/// assert_eq!(*input_graph.g.node_weight(input_node_1).unwrap().data().clone().unwrap(), "A");
/// assert_eq!(*input_graph.g.node_weight(input_node_2).unwrap().data().clone().unwrap(), "B");
///
/// assert_eq!(*input_graph.g.edge_weight(input_graph.g.edge_indices().next().unwrap()).unwrap().data().clone().unwrap(), "edge1");
///
/// assert_eq!(*input_graph.g.node_weight(input_node_1).unwrap().label().clone(), input_node_1.index().to_string());
/// assert_eq!(*input_graph.g.node_weight(input_node_2).unwrap().label().clone(), input_node_2.index().to_string());
///
/// let loc_1 = input_graph.g.node_weight(input_node_1).unwrap().location();
/// let loc_2 = input_graph.g.node_weight(input_node_2).unwrap().location();
/// assert!(loc_1 != Vec2::ZERO);
/// assert!(loc_2 != Vec2::ZERO);
/// ```
pub fn to_graph<N: Clone, E: Clone, Ty: EdgeType>(g: &StableGraph<N, E, Ty>) -> Graph<N, E, Ty> {
    transform(g, default_node_transform, default_edge_transform)
}

/// The same as [`to_graph`], but allows to define custom transformation procedures for nodes and edges.
pub fn to_graph_custom<N: Clone, E: Clone, Ty: EdgeType>(
    g: &StableGraph<N, E, Ty>,
    node_transform: impl Fn(NodeIndex, &N) -> Node<N>,
    edge_transform: impl Fn(EdgeIndex, &E) -> Edge<E>,
) -> Graph<N, E, Ty> {
    transform(g, node_transform, edge_transform)
}

/// Default node transform function. Keeps original data and creates a new node with a random location and
/// label equal to the index of the node in the graph.
pub fn default_node_transform<N: Clone>(idx: NodeIndex, data: &N) -> Node<N> {
    let loc = random_location(DEFAULT_SPAWN_SIZE);
    Node::new(loc, data.clone()).with_label(idx.index().to_string())
}

/// Default edge transform function. Keeps original data and creates a new edge.
pub fn default_edge_transform<E: Clone>(_: EdgeIndex, data: &E) -> Edge<E> {
    Edge::new(data.clone())
}

fn random_location(size: f32) -> Vec2 {
    let mut rng = rand::thread_rng();
    Vec2::new(rng.gen_range(0. ..size), rng.gen_range(0. ..size))
}

fn transform<N: Clone, E: Clone, Ty: EdgeType>(
    g: &StableGraph<N, E, Ty>,
    node_transform: impl Fn(NodeIndex, &N) -> Node<N>,
    edge_transform: impl Fn(EdgeIndex, &E) -> Edge<E>,
) -> Graph<N, E, Ty> {
    let mut input_g = StableGraph::<Node<N>, Edge<E>, Ty>::default();

    let input_by_user = g
        .node_references()
        .map(|(user_n_idx, user_n)| {
            let input_n_index = input_g.add_node(node_transform(user_n_idx, user_n));
            (user_n_idx, input_n_index)
        })
        .collect::<HashMap<NodeIndex, NodeIndex>>();

    g.edge_indices().for_each(|user_e_idx| {
        let (user_source_n_idx, user_target_n_idx) = g.edge_endpoints(user_e_idx).unwrap();
        let user_e = g.edge_weight(user_e_idx).unwrap();

        let input_source_n = *input_by_user.get(&user_source_n_idx).unwrap();
        let input_target_n = *input_by_user.get(&user_target_n_idx).unwrap();

        input_g.add_edge(
            input_source_n,
            input_target_n,
            edge_transform(user_e_idx, user_e),
        );
    });

    Graph::new(input_g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::Directed;
    use petgraph::Undirected;

    #[test]
    fn test_to_graph_directed() {
        let mut user_g: StableGraph<_, _, Directed> = StableGraph::new();
        let n1 = user_g.add_node("Node1");
        let n2 = user_g.add_node("Node2");
        user_g.add_edge(n1, n2, "Edge1");

        let input_g = to_graph(&user_g);

        assert_eq!(user_g.node_count(), input_g.g.node_count());
        assert_eq!(user_g.edge_count(), input_g.g.edge_count());
        assert_eq!(user_g.is_directed(), input_g.is_directed());

        for (user_idx, input_idx) in input_g.g.node_indices().zip(user_g.node_indices()) {
            let user_n = user_g.node_weight(user_idx).unwrap();
            let input_n = input_g.g.node_weight(input_idx).unwrap();

            assert_eq!(input_n.data, *user_n);

            assert!(input_n.location().x >= 0.0 && input_n.location().x <= DEFAULT_SPAWN_SIZE);
            assert!(input_n.location().y >= 0.0 && input_n.location().y <= DEFAULT_SPAWN_SIZE);

            assert_eq!(*input_n.label(), user_idx.index().to_string());

            assert!(!input_n.selected());
            assert!(!input_n.dragged());
        }
    }

    #[test]
    fn test_to_graph_undirected() {
        let mut user_g: StableGraph<_, _, Undirected> = StableGraph::default();
        let n1 = user_g.add_node("Node1");
        let n2 = user_g.add_node("Node2");
        user_g.add_edge(n1, n2, "Edge1");

        let input_g = to_graph(&user_g);

        assert_eq!(user_g.node_count(), input_g.g.node_count());
        assert_eq!(user_g.edge_count(), input_g.g.edge_count());
        assert_eq!(user_g.is_directed(), input_g.is_directed());

        for (user_idx, input_idx) in input_g.g.node_indices().zip(user_g.node_indices()) {
            let user_n = user_g.node_weight(user_idx).unwrap();
            let input_n = input_g.g.node_weight(input_idx).unwrap();

            assert_eq!(input_n.data, *user_n);

            assert!(input_n.location().x >= 0.0 && input_n.location().x <= DEFAULT_SPAWN_SIZE);
            assert!(input_n.location().y >= 0.0 && input_n.location().y <= DEFAULT_SPAWN_SIZE);

            assert_eq!(*input_n.label(), user_idx.index().to_string());

            assert!(!input_n.selected());
            assert!(!input_n.dragged());
        }
    }
}
