use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::evan::ROOT_ID;

pub mod evan;
pub mod martin;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TreeNode {
    id: NodeID,
    children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn from_state(state: &HashMap<NodeID, Option<NodeID>>) -> TreeNode {
        let root_id = state
            .iter()
            .find_map(|(id, parent)| if parent.is_none() { Some(*id) } else { None })
            .expect("No root node found");

        TreeNode::build_tree(root_id, state)
    }

    fn build_tree(node_id: NodeID, state: &HashMap<NodeID, Option<NodeID>>) -> TreeNode {
        let mut children = state
            .iter()
            .filter_map(|(id, parent)| {
                if Some(node_id) == *parent {
                    Some(TreeNode::build_tree(*id, state))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        children.sort();
        TreeNode {
            id: node_id,
            children,
        }
    }
}

impl Display for NodeID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if *self == ROOT_ID {
            return write!(f, "ROOT");
        }
        write!(f, "Node[ {}@{} ]", self.lamport, self.peer)
    }
}

impl TreeNode {
    fn to_string(&self, prefix: String, last: bool) -> String {
        let connector = if last { "└── " } else { "├── " };
        let mut s = format!("{}{}{}\n", prefix, connector, self.id);

        let new_prefix = if last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        let len = self.children.len();
        for (i, child) in self.children.iter().enumerate() {
            s += child.to_string(new_prefix.clone(), i + 1 == len).as_str();
        }
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ID {
    pub lamport: u32,
    pub peer: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeID {
    pub lamport: u32,
    pub peer: u64,
}

impl From<ID> for NodeID {
    fn from(id: ID) -> Self {
        NodeID {
            lamport: id.lamport,
            peer: id.peer,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TreeOp {
    Create { parent: NodeID },
    Move { target: NodeID, parent: NodeID },
}

#[derive(Debug, Clone, Copy)]
pub struct Op {
    id: ID,
    op: TreeOp,
}

impl PartialEq for Op {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Op {}

impl Ord for Op {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Op {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

pub trait MovableTreeAlgorithm {
    fn new() -> Self;
    fn apply(&mut self, op: Op) -> Option<NodeID>;
    fn merge(&mut self, ops: Vec<Op>);
    fn nodes(&self) -> Vec<NodeID>;
    fn parent(&self, node: NodeID) -> Option<NodeID>;
    fn get_root(&self) -> TreeNode;
    fn is_ancestor_of(&self, maybe_ancestor: NodeID, mut node_id: NodeID) -> bool {
        if maybe_ancestor == node_id {
            return true;
        }

        loop {
            let parent = self.parent(node_id);
            match parent {
                Some(parent_id) if parent_id == maybe_ancestor => return true,
                Some(parent_id) if parent_id == node_id => panic!("loop detected"),
                Some(parent_id) => {
                    node_id = parent_id;
                }
                None => return false,
            }
        }
    }
}

pub struct MovableTree<T> {
    algorithm: T,
    peer: u64,
    ops: HashMap<u64, Vec<Op>>,
    next_lamport: u32,
}

impl<T: MovableTreeAlgorithm> MovableTree<T> {
    pub fn new(peer: u64) -> Self {
        MovableTree {
            algorithm: T::new(),
            ops: HashMap::default(),
            peer,
            next_lamport: 0,
        }
    }

    pub fn new_id(&mut self) -> ID {
        let id = ID {
            lamport: self.next_lamport,
            peer: self.peer,
        };
        self.next_lamport += 1;
        id
    }

    pub fn create(&mut self, parent: NodeID) -> NodeID {
        let id = self.new_id();
        let op = Op {
            id,
            op: TreeOp::Create { parent },
        };
        self.ops.entry(self.peer).or_default().push(op);
        self.algorithm.apply(op).unwrap()
    }

    #[allow(clippy::result_unit_err)]
    pub fn mov(&mut self, target: NodeID, parent: NodeID) -> Result<(), ()> {
        if self.algorithm.is_ancestor_of(target, parent) {
            return Err(());
        }
        let op = Op {
            id: self.new_id(),
            op: TreeOp::Move { target, parent },
        };
        self.ops.entry(self.peer).or_default().push(op);
        self.algorithm.apply(op);
        Ok(())
    }

    pub fn merge(&mut self, other: &Self) {
        let mut ans = Vec::new();
        for (peer, ops) in other.ops.iter() {
            let self_start = self.ops.get(peer).map(|v| v.len()).unwrap_or(0);
            if ops.len() > self_start {
                let entry = self.ops.entry(*peer).or_default();
                for &op in &ops[self_start..] {
                    entry.push(op);
                    ans.push(op);
                    if op.id.lamport >= self.next_lamport {
                        self.next_lamport = op.id.lamport + 1;
                    }
                }
            }
        }
        self.algorithm.merge(ans);
    }
}

impl<T: MovableTreeAlgorithm> ToString for MovableTree<T> {
    fn to_string(&self) -> String {
        let root = self.algorithm.get_root();
        root.to_string("".to_string(), true)
    }
}
