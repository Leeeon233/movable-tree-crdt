use fxhash::{FxHashMap, FxHashSet};
use std::collections::{hash_map::Entry, BinaryHeap};

use crate::{MovableTreeAlgorithm, NodeID, Op, TreeNode, TreeOp, ROOT_ID};

#[derive(Debug, Clone, Copy)]
struct EdgeCounter {
    counter: u32,
    lamport: u32,
    peer: u64,
}

#[derive(Debug, Clone)]
pub struct Node {
    id: NodeID,
    parent: Option<NodeID>,
    edges: FxHashMap<NodeID, EdgeCounter>,
}

impl Node {
    pub fn largest_edge(&self) -> Option<NodeID> {
        self.edges
            .iter()
            .max_by(
                |(a_id, EdgeCounter { counter: a_c, .. }),
                 (b_id, EdgeCounter { counter: b_c, .. })| {
                    a_c.cmp(b_c).then_with(|| a_id.cmp(b_id))
                },
            )
            .map(|(id, _)| *id)
    }
}

pub struct EvanTree {
    pub nodes: FxHashMap<NodeID, Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PQItem {
    child: NodeID,
    parent: NodeID,
    counter: u32,
}

impl PartialOrd for PQItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PQItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.counter
            .cmp(&other.counter)
            .then_with(|| self.parent.cmp(&other.parent))
            .then_with(|| self.child.cmp(&other.child))
    }
}

impl Default for EvanTree {
    fn default() -> Self {
        let root = Node {
            id: ROOT_ID,
            parent: None,
            edges: FxHashMap::default(),
        };
        let mut nodes = FxHashMap::default();
        nodes.insert(root.id, root);
        EvanTree { nodes }
    }
}

impl EvanTree {
    pub fn new() -> Self {
        Self::default()
    }

    fn recompute_parent_children(&mut self) {
        // Start off with all children arrays empty and each parent pointer
        // for a given node set to the most recent edge for that node.
        self.nodes.values_mut().for_each(|node| {
            node.parent = node.largest_edge();
            // node.children.clear();
        });
        // At this point all nodes that can reach the root form a tree (by
        // construction, since each node other than the root has a single
        // parent). The parent pointers for the remaining nodes may form one
        // or more cycles. Gather all remaining nodes detached from the root.
        let mut non_rooted_nodes = FxHashSet::default();
        for node in self.nodes.values() {
            if !non_rooted_nodes.contains(&node.id) && !self.is_under_other(node.id, ROOT_ID) {
                let mut node_id = Some(node.id);
                while let Some(node) = node_id {
                    if !non_rooted_nodes.contains(&node) {
                        non_rooted_nodes.insert(node);
                        node_id = self.parent(node);
                    } else {
                        break;
                    }
                }
            }
        }
        // Deterministically reattach these nodes to the tree under the root
        // node. The order of reattachment is arbitrary but needs to be based
        // only on information in the database so that all peers reattach
        // non-rooted nodes in the same order and end up with the same tree.
        if !non_rooted_nodes.is_empty() {
            // All "ready" edges already have the parent connected to the root,
            // and all "deferred" edges have a parent not yet connected to the
            // root. Prioritize newer edges over older edges using the counter.
            let mut deferred_edges = FxHashMap::default();
            let mut ready_edges = BinaryHeap::new();
            for &child in non_rooted_nodes.iter() {
                for (&parent, &counter) in self.nodes.get(&child).unwrap().edges.iter() {
                    if !non_rooted_nodes.contains(&parent) {
                        ready_edges.push(PQItem {
                            child,
                            parent,
                            counter: counter.counter,
                        });
                    } else {
                        deferred_edges
                            .entry(parent)
                            .or_insert_with(Vec::new)
                            .push(PQItem {
                                child,
                                parent,
                                counter: counter.counter,
                            });
                    }
                }
            }
            while let Some(top) = ready_edges.pop() {
                let child = top.child;
                if !non_rooted_nodes.contains(&child) {
                    continue;
                }

                // reattach child to parent
                self.nodes.get_mut(&child).unwrap().parent = Some(top.parent);
                non_rooted_nodes.remove(&child);

                // active all deferred edges for child
                if let Some(deferred) = deferred_edges.remove(&child) {
                    for edge in deferred {
                        ready_edges.push(edge);
                    }
                }
            }
        }
    }

    pub fn is_under_other(&self, node: NodeID, other: NodeID) -> bool {
        if node == other {
            return true;
        }
        let mut tortoise = node;
        let mut hare = self.parent(node);
        while hare.is_some() && hare.unwrap() != other {
            if tortoise == hare.unwrap() {
                return false;
            }
            hare = self.parent(hare.unwrap());
            if hare.is_none() || hare.unwrap() == other {
                break;
            }
            tortoise = self.parent(tortoise).unwrap();
            hare = self.parent(hare.unwrap());
        }
        hare == Some(other)
    }

    fn ensure_node_is_rooted(
        &mut self,
        mut node: Option<NodeID>,
        edits: &mut Vec<(NodeID, NodeID)>,
    ) {
        while let Some(child) = node.and_then(|id| self.nodes.get(&id)) {
            let parent = child.parent;
            if parent.is_none() {
                break;
            }
            let edge = child.largest_edge();
            if edge != parent {
                edits.push((node.unwrap(), parent.unwrap()));
            }
            node = parent;
        }
    }
}

impl MovableTreeAlgorithm for EvanTree {
    fn new() -> Self {
        Self::new()
    }

    fn apply(&mut self, op: Op, local: bool) -> Vec<Op> {
        let id = op.id;
        match op.op {
            TreeOp::Create { parent } => {
                let child = self.nodes.entry(id.into()).or_insert_with(|| Node {
                    id: id.into(),
                    parent: Some(parent),
                    // children: vec![],
                    edges: FxHashMap::default(),
                });
                child.edges.insert(
                    parent,
                    EdgeCounter {
                        counter: 0,
                        lamport: id.lamport,
                        peer: id.peer,
                    },
                );
                vec![op]
            }
            TreeOp::Move {
                target,
                parent,
                counter,
            } => {
                if local {
                    let child = target;
                    let mut edits = vec![];
                    let old_parent = self.parent(child);
                    self.ensure_node_is_rooted(old_parent, &mut edits);
                    self.ensure_node_is_rooted(Some(parent), &mut edits);
                    edits.push((child, parent));
                    let mut ans = Vec::with_capacity(edits.len());
                    for (child, parent) in edits {
                        let max_counter = self
                            .nodes
                            .get(&child)
                            .unwrap()
                            .edges
                            .values()
                            .map(|c| c.counter as i64)
                            .max()
                            .unwrap_or(-1);
                        self.nodes.get_mut(&child).unwrap().edges.insert(
                            parent,
                            EdgeCounter {
                                counter: (max_counter + 1) as u32,
                                lamport: id.lamport,
                                peer: id.peer,
                            },
                        );
                        ans.push(Op {
                            id,
                            op: TreeOp::Move {
                                target,
                                parent,
                                counter: (max_counter + 1) as u32,
                            },
                        })
                    }
                    self.recompute_parent_children();
                    ans
                } else {
                    let child = target;
                    let edge = self.nodes.get_mut(&child).unwrap().edges.entry(parent);
                    match edge {
                        Entry::Occupied(mut entry) => {
                            let old_counter = entry.get_mut();
                            if old_counter.lamport < id.lamport
                                || (old_counter.lamport == id.lamport && old_counter.peer < id.peer)
                            {
                                old_counter.counter = counter;
                                old_counter.lamport = id.lamport;
                                old_counter.peer = id.peer;
                            }
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(EdgeCounter {
                                counter,
                                lamport: id.lamport,
                                peer: id.peer,
                            });
                        }
                    }
                    vec![]
                }
            }
        }
    }

    fn merge(&mut self, ops: Vec<Op>) {
        for op in ops {
            self.apply(op, false);
        }
        self.recompute_parent_children()
    }

    fn nodes(&self) -> Vec<NodeID> {
        self.nodes.keys().copied().collect()
    }

    fn parent(&self, node: NodeID) -> Option<NodeID> {
        self.nodes.get(&node).and_then(|n| n.parent)
    }

    fn get_root(&self) -> TreeNode {
        let state = self.nodes.iter().map(|(&k, v)| (k, v.parent)).collect();
        TreeNode::from_state(&state)
    }
}
