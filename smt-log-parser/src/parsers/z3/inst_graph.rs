use fxhash::FxHashMap;
use petgraph::{
    stable_graph::{EdgeIndex, StableGraph},
    visit::{Dfs, EdgeRef},
    Direction::{Incoming, Outgoing},
};
// use gloo_console::log;
use petgraph::graph::NodeIndex;
use std::fmt;

use crate::items::{InstIdx, QuantIdx, TermIdx};

use super::z3parser::Z3Parser;

#[derive(Clone, Copy)]
pub struct NodeData {
    pub line_nr: usize,
    pub is_theory_inst: bool,
    cost: f32,
    inst_idx: Option<InstIdx>,
    pub quant_idx: QuantIdx,
    remove: bool,
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line_nr)
    }
}

#[derive(PartialEq, Clone)]
pub struct InstInfo {
    pub line_no: usize,
    pub cost: f32,
    pub formula: String,
    pub bound_terms: Vec<String>,
    pub yields_terms: Vec<String>,
    pub node_index: NodeIndex,
}

#[derive(Default, Clone)]
pub struct InstGraph {
    pub inst_graph: StableGraph<NodeData, ()>, // same as orig_inst_graph but possibly filtered
    orig_graph: StableGraph<NodeData, ()>,
    node_of_line_nr: FxHashMap<usize, NodeIndex>, // line number => node-index
}

impl InstGraph {
    pub fn from(parser: &Z3Parser) -> Self {
        let mut inst_graph = Self::default();
        inst_graph.compute_instantiation_graph(parser);
        inst_graph
    }

    pub fn retain_nodes(&mut self, retain: impl Fn(&NodeData) -> bool) {
        self.inst_graph
            .retain_nodes(|graph, node_idx| retain(graph.node_weight(node_idx).unwrap()))
    }

    pub fn retain_nodes_and_reconnect(&mut self, retain: impl Fn(&NodeData) -> bool) {
        let nodes_to_remove: Vec<NodeIndex> = self
            .inst_graph
            .node_indices()
            .filter(|&node_idx| !retain(self.inst_graph.node_weight(node_idx).unwrap()))
            .collect();
        for node in nodes_to_remove {
            let preds: Vec<NodeIndex> =
                self.inst_graph.neighbors_directed(node, Incoming).collect();
            let succs: Vec<NodeIndex> =
                self.inst_graph.neighbors_directed(node, Outgoing).collect();
            self.inst_graph.remove_node(node);
            for &pred in &preds {
                for &succ in &succs {
                    self.inst_graph.add_edge(pred, succ, ());
                }
            }
        }
    }

    pub fn keep_n_most_costly(&mut self, n: usize) {
        // Only keep the max_instantiations most costly instantiations by sorting in
        // descending order of the cost.
        // In case two instantiations have the same cost, the instantiation with the lower
        // line number comes first in the order (is greater), or mathematically:
        // This is a total order since the line numbers are always guaranteed to be distinct
        // integers.
        // inst_b > inst_a iff (cost_b > cost_a or (cost_b = cost_a and line_nr_b < line_nr_a))
        let mut most_costly_insts: Vec<NodeIndex> = self.inst_graph.node_indices().collect();
        most_costly_insts.sort_by(|node_a, node_b| {
            let node_a_data = self.inst_graph.node_weight(*node_a).unwrap();
            let node_b_data = self.inst_graph.node_weight(*node_b).unwrap();
            if node_a_data.cost < node_b_data.cost {
                std::cmp::Ordering::Greater
            } else if node_a_data.cost == node_b_data.cost
                && node_b_data.line_nr < node_a_data.line_nr
            {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });
        most_costly_insts.truncate(n);
        self.inst_graph
            .retain_nodes(|_, node| most_costly_insts.contains(&node));
    }

    pub fn remove_subtree_with_root(&mut self, root: NodeIndex) {
        let mut dfs = Dfs::new(&self.inst_graph, root);
        // iterate through all descendants of root and mark them to be removed
        while let Some(nx) = dfs.next(&self.inst_graph) {
            self.inst_graph[nx].remove = true;
        }
        // remove the marked nodes
        self.inst_graph
            .retain_nodes(|graph, node| !graph.node_weight(node).unwrap().remove)
    }

    pub fn only_show_ancestors(&mut self, node: NodeIndex) {
        // create new graph which is identical to original one except that all nodes have
        // remove = true instead of remove = false
        let mut ancestors = self.orig_graph.map(
            |_, &node| {
                let mut hidden_node = node;
                hidden_node.remove = true;
                hidden_node
            },
            |_, _| (),
        );
        // visit all ancestors of node (argument) and set their remove-field to false since we want to retain them
        let orig_with_reversed_edges = petgraph::visit::Reversed(&self.orig_graph);
        let mut dfs = Dfs::new(orig_with_reversed_edges, node);
        while let Some(nx) = dfs.next(orig_with_reversed_edges) {
            ancestors[nx].remove = false;
        }
        // retain all ancestors of node, i.e., where remove-field was previously set to true
        ancestors.retain_nodes(|graph, node| !graph.node_weight(node).unwrap().remove);
        self.inst_graph = ancestors
    }

    pub fn reset(&mut self) {
        self.inst_graph = self.orig_graph.clone();
    }

    pub fn show_neighbours(&mut self, node: NodeIndex, direction: petgraph::Direction) {
        let neighbours: Vec<NodeIndex> = self
            .orig_graph
            .neighbors_directed(node, direction)
            .collect();
        let edges_to_neighbours: Vec<EdgeIndex> = self
            .orig_graph
            .edges_directed(node, direction)
            .map(|e| e.id())
            .collect();
        let new_inst_graph = self.orig_graph.filter_map(
            |node, _| {
                if self.inst_graph.contains_node(node) || neighbours.contains(&node) {
                    Some(self.orig_graph[node])
                } else {
                    None
                }
            },
            |edge, _| {
                if self.inst_graph.edge_indices().any(|e| e == edge)
                    || edges_to_neighbours.contains(&edge)
                {
                    Some(())
                } else {
                    None
                }
            },
        );
        self.inst_graph = new_inst_graph;
    }

    pub fn node_count(&self) -> usize {
        self.inst_graph.node_count()
    }

    pub fn get_instantiation_info(&self, node_index: usize, parser: &Z3Parser) -> Option<InstInfo> {
        let NodeData { inst_idx, .. } = self
            .inst_graph
            .node_weight(NodeIndex::new(node_index))
            .unwrap();
        if let Some(iidx) = inst_idx {
            let inst = parser.instantiations.get(*iidx).unwrap();
            let quant = parser.quantifiers.get(inst.quant).unwrap();
            let term_map = &parser.terms;
            let pretty_text_map = |tidxs: &Vec<TermIdx>| {
                tidxs
                    .iter()
                    .map(|tidx| term_map.get(*tidx).unwrap())
                    .map(|term| term.pretty_text(term_map))
                    .collect::<Vec<String>>()
            };
            let bound_terms = pretty_text_map(&inst.bound_terms);
            let yields_terms = pretty_text_map(&inst.yields_terms);
            let inst_info = InstInfo {
                line_no: inst.line_no.unwrap(),
                cost: inst.cost,
                formula: quant.pretty_text(term_map),
                bound_terms,
                yields_terms,
                node_index: NodeIndex::new(node_index),
            };
            Some(inst_info)
        } else {
            None
        }
    }

    fn compute_instantiation_graph(&mut self, parser: &Z3Parser) {
        for dep in &parser.dependencies {
            if let Some(to) = dep.to {
                let quant_idx = dep.quant;
                let quant = parser.quantifiers.get(quant_idx).unwrap();
                let cost = quant.cost;
                self.add_node(NodeData {
                    line_nr: to,
                    is_theory_inst: dep.quant_discovered,
                    cost,
                    inst_idx: dep.to_iidx,
                    quant_idx,
                    remove: false,
                });
            }
        }
        // then add all edges between nodes
        for dep in &parser.dependencies {
            let from = dep.from;
            if let Some(to) = dep.to {
                if from > 0 {
                    self.add_edge(from, to);
                }
            }
        }
    }

    fn fresh_line_nr(&self, line_nr: usize) -> bool {
        // self.orig_inst_graph.node_weights().all(|&line| line != line_nr)
        self.inst_graph
            .node_weights()
            .all(|node| node.line_nr != line_nr)
    }

    fn add_node(&mut self, node_data: NodeData) {
        let line_nr = node_data.line_nr;
        if self.fresh_line_nr(line_nr) {
            let node = self.inst_graph.add_node(node_data);
            self.orig_graph.add_node(node_data);
            self.node_of_line_nr.insert(line_nr, node);
        }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (
            self.node_of_line_nr.get(&from),
            self.node_of_line_nr.get(&to),
        ) {
            self.inst_graph.add_edge(from_node_idx, to_node_idx, ());
            self.orig_graph.add_edge(from_node_idx, to_node_idx, ());
        }
    }
}
