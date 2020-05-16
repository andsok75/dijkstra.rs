use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::iter::Sum;
use std::ops::Add;
use std::collections::HashMap;

mod priority_queue;

// data-oriented graph with user-defined node states and edge props;
// nodes and edges can be inserted but not deleted
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Graph<NodeState: Debug, EdgeProps: Debug> {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    states: Vec<NodeState>,
    props: Vec<EdgeProps>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Node {
    pub id: NodeId,
    pub incoming: Vec<EdgeId>,
    pub outgoing: Vec<EdgeId>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
}

pub trait Cost {
    type Type: Debug + Copy + PartialOrd + PartialEq + Ord + Add<Output = Self::Type> + Sum;
    fn cost(&self) -> Self::Type;
    fn zero_cost() -> Self::Type;
}

type NodeId = usize;
type EdgeId = usize;

impl<NodeState: Debug, EdgeProps: Debug + Cost> Graph<NodeState, EdgeProps> {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            states: Vec::new(),
            edges: Vec::new(),
            props: Vec::new(),
        }
    }
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }
    pub fn state(&self, id: NodeId) -> &NodeState {
        &self.states[id]
    }
    pub fn edge(&self, id: EdgeId) -> &Edge {
        &self.edges[id]
    }
    pub fn props(&self, id: EdgeId) -> &EdgeProps {
        &self.props[id]
    }
    pub fn cost(&self, path: &[EdgeId]) -> <EdgeProps as Cost>::Type {
        path.iter()
            .cloned()
            .map(|edge_id| self.props[edge_id].cost())
            .sum()
    }
    pub fn insert_node(&mut self, state: NodeState) -> NodeId {
        let new_node_id = self.nodes.len();
        self.nodes.push(Node {
            id: new_node_id,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        });
        self.states.push(state);
        new_node_id
    }
    pub fn insert_edge(&mut self, from: NodeId, to: NodeId, props: EdgeProps) -> EdgeId {
        let new_edge_id = self.edges.len();
        self.edges.push(Edge {
            id: new_edge_id,
            from,
            to,
        });
        self.props.push(props);
        self.nodes[from].outgoing.push(new_edge_id);
        self.nodes[to].incoming.push(new_edge_id);
        new_edge_id
    }
    // find the cheapest path to any of the targets
    pub fn best_path(&self, source: NodeId, targets: &[NodeId]) -> Option<Vec<EdgeId>> {
        if targets.contains(&source) {
            return Some(Vec::new());
        }
        // from the source, use breadth-first search to find the cheapest incoming edge for each node
        let mut best_incoming = vec![None; self.nodes.len()];
        let mut best_cost = vec![None; self.nodes.len()];
        let mut is_closed = vec![false; self.nodes.len()];
        let mut queue = priority_queue::Heap::<<EdgeProps as Cost>::Type>::new();
        queue.insert(source, EdgeProps::zero_cost());
        while !queue.is_empty() {
            let (from, from_cost) = queue.extract_min().unwrap();
            is_closed[from] = true;
            for &edge_id in self.nodes[from].outgoing.iter() {
                let to = self.edges[edge_id].to;
                if to == from || is_closed[to] {
                    // skip loopy edges (they just increase cost) or edges that end at a closed node,
                    // since we're using priority queue and thus a closed node already has the best cost and incoming
                    continue;
                }
                let to_cost = from_cost + self.props[edge_id].cost();
                if best_cost[to].is_none() || to_cost < best_cost[to].unwrap() {
                    best_cost[to] = Some(to_cost);
                    best_incoming[to] = Some(edge_id);
                    queue.insert(to, to_cost);
                    // the queue might still have the old more expensive items for 'to',
                    // but they will be discarded when they eventually get to the front of the queue
                }
            }
        }
        // then find the cheapest path walking back from the cheapest target via the cheapest incoming edges
        let cheapest_target: Option<NodeId> = targets
            .iter()
            .cloned()
            .filter(|&target| best_cost[target].is_some())
            .min_by_key(|&target| best_cost[target].unwrap());
        let mut node_id = cheapest_target?;
        let mut path = Vec::new();
        while node_id != source {
            if let Some(edge_id) = best_incoming[node_id] {
                path.push(edge_id);
                node_id = self.edges[edge_id].from;
            } else {
                unreachable!();
            }
        }
        path.reverse();
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct State {
        name: char,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Props {
        cost: u8,
    }

    impl Cost for Props {
        type Type = u8;
        fn cost(&self) -> Self::Type {
            self.cost
        }
        fn zero_cost() -> Self::Type {
            0u8
        }
    }

    fn graph_from_edges(edges: &[(char, char, u8)]) -> (Graph<State, Props>, impl Fn(char) -> usize) {
        let mut graph: Graph<State, Props> = Graph::new();
        let mut id = HashMap::new();
        for &(from_name, to_name, cost) in edges.iter() {
            id.entry(from_name).or_insert(graph.insert_node(State { name: from_name }));
            id.entry(to_name).or_insert(graph.insert_node(State { name: to_name }));
            graph.insert_edge(*id.get(&from_name).unwrap(), *id.get(&to_name).unwrap(), Props { cost });
        }
        (graph, move |name| *id.get(&name).unwrap())
    }

    #[test]
    fn test() {
        let (graph, node_id) = graph_from_edges(&[
            ('a', 'b', 1),
            ('b', 'd', 10),
            ('a', 'c', 2),
            ('c', 'b', 3),
            ('c', 'd', 8),
            ]);
        let from = |edge_id| graph.state(graph.edge(edge_id).from).name;
        let to   = |edge_id| graph.state(graph.edge(edge_id).to).name;

        assert_eq!(node_id('a'), 0);
        let path = graph.best_path(node_id('a'), &[node_id('d')]).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(from(path[0]), 'a');
        assert_eq!(to(path[0]), 'c');
        assert_eq!(from(path[1]), 'c');
        assert_eq!(to(path[1]), 'd');
        assert_eq!(graph.cost(&path), 10);
    }
}
