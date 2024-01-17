use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::{MovableTreeAlgorithm, NodeID, Op, TreeNode, TreeOp, ID};

pub const ROOT_ID: NodeID = NodeID {
    lamport: u32::MAX,
    peer: u64::MAX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EdgeCounter(u32);

#[derive(Debug, Clone)]
pub struct Node {
    id: NodeID,
    parent: Option<NodeID>,
    children: Vec<NodeID>,
    edges: HashMap<NodeID, EdgeCounter>,
}

impl Node {
    pub fn largest_edge(&self) -> Option<NodeID> {
        self.edges
            .iter()
            .max_by(|(a_id, EdgeCounter(a_c)), (b_id, EdgeCounter(b_c))| {
                a_c.cmp(b_c).then_with(|| a_id.cmp(b_id))
            })
            .map(|(id, _)| *id)
    }
}

pub struct EvanTree {
    nodes: HashMap<NodeID, Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PQItem {
    child: NodeID,
    parent: NodeID,
    counter: EdgeCounter,
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
            children: Vec::new(),
            edges: HashMap::new(),
        };
        let mut nodes = HashMap::new();
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
            node.children.clear();
        });
        // At this point all nodes that can reach the root form a tree (by
        // construction, since each node other than the root has a single
        // parent). The parent pointers for the remaining nodes may form one
        // or more cycles. Gather all remaining nodes detached from the root.
        let mut non_rooted_nodes = HashSet::new();
        for node in self.nodes.values() {
            if !self.is_under_other(node.id, self.root()) {
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
            let mut deferred_edges = HashMap::new();
            let mut ready_edges = BinaryHeap::new();
            for &child in non_rooted_nodes.iter() {
                for (&parent, &counter) in self.nodes.get(&child).unwrap().edges.iter() {
                    if !non_rooted_nodes.contains(&parent) {
                        ready_edges.push(PQItem {
                            child,
                            parent,
                            counter,
                        });
                    } else {
                        deferred_edges
                            .entry(parent)
                            .or_insert_with(Vec::new)
                            .push(PQItem {
                                child,
                                parent,
                                counter,
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

        // Add items as children of their parents so that the rest of the app
        // can easily traverse down the tree for drawing and hit-testing
        for (id, parent) in self
            .nodes
            .values()
            .map(|n| (n.id, n.parent))
            .collect::<Vec<_>>()
        {
            if let Some(parent) = parent {
                self.nodes.get_mut(&parent).unwrap().children.push(id);
            }
        }

        self.nodes.values_mut().for_each(|n| n.children.sort());
    }

    pub fn is_under_other(&self, node: NodeID, other: NodeID) -> bool {
        if node == other {
            return true;
        }
        let mut tortoise = node;
        let mut hare = self.parent_node(node);
        while hare.is_some() && hare.unwrap().id != other {
            if tortoise == hare.unwrap().id {
                return false;
            }
            hare = hare.unwrap().parent.and_then(|id| self.nodes.get(&id));
            if hare.is_none() || hare.unwrap().id == other {
                break;
            }
            tortoise = self.nodes.get(&tortoise).unwrap().parent.unwrap();
            hare = hare.unwrap().parent.and_then(|id| self.nodes.get(&id));
        }
        hare.map(|n| n.id) == Some(other)
    }

    fn parent_node(&self, node: NodeID) -> Option<&Node> {
        self.parent(node).and_then(|id| self.nodes.get(&id))
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

    fn mov(&mut self, target: NodeID, parent: NodeID) {
        let child = target;
        let mut edits = vec![];
        let old_parent = self.parent(child);
        self.ensure_node_is_rooted(old_parent, &mut edits);
        self.ensure_node_is_rooted(Some(parent), &mut edits);
        edits.push((child, parent));

        for (child, parent) in edits {
            let max_counter = self
                .nodes
                .get(&child)
                .unwrap()
                .edges
                .values()
                .map(|c| c.0 as i64)
                .max()
                .unwrap_or(-1);
            self.nodes
                .get_mut(&child)
                .unwrap()
                .edges
                .insert(parent, EdgeCounter((max_counter + 1) as u32));
            self.recompute_parent_children();
        }
    }

    fn create(&mut self, id: ID, parent: NodeID) -> NodeID {
        let child = self.nodes.entry(id.into()).or_insert_with(|| Node {
            id: id.into(),
            parent: None,
            children: vec![],
            edges: HashMap::new(),
        });
        child.edges.insert(parent, EdgeCounter(0));
        self.recompute_parent_children();
        id.into()
    }
}

impl MovableTreeAlgorithm for EvanTree {
    fn new() -> Self {
        Self::new()
    }

    fn apply(&mut self, op: Op) -> Option<NodeID> {
        match op.op {
            TreeOp::Create { parent } => Some(self.create(op.id, parent)),
            TreeOp::Move { target, parent } => {
                self.mov(target, parent);
                None
            }
        }
    }

    fn merge(&mut self, ops: Vec<Op>) {
        for op in ops {
            self.apply(op);
        }
    }

    fn nodes(&self) -> Vec<NodeID> {
        self.nodes.keys().copied().collect()
    }

    fn parent(&self, node: NodeID) -> Option<NodeID> {
        self.nodes.get(&node).and_then(|n| n.parent)
    }

    fn children(&self, node: NodeID) -> Vec<TreeNode> {
        self.nodes
            .get(&node)
            .map(|n| {
                n.children
                    .iter()
                    .map(|id| TreeNode {
                        id: *id,
                        children: self.children(*id),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn root(&self) -> NodeID {
        ROOT_ID
    }

    fn get_node(&self, node: NodeID) -> Option<TreeNode> {
        self.nodes.get(&node).map(|n| TreeNode {
            id: n.id,
            children: self.children(node),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    #[test]
    fn test() {
        let mut tree = EvanTree::new();
        let child = tree.create(
            ID {
                lamport: 0,
                peer: 0,
            },
            ROOT_ID,
        );
        let child2 = tree.create(
            ID {
                lamport: 1,
                peer: 0,
            },
            ROOT_ID,
        );
        tree.mov(child, child2);
        println!("{:?}", tree.nodes())
    }
}
