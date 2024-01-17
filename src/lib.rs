use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::evan::ROOT_ID;

pub mod evan;

#[derive(Debug)]
pub struct TreeNode {
    id: NodeID,
    children: Vec<TreeNode>,
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

pub trait MovableTreeAlgorithm {
    fn new() -> Self;
    fn apply(&mut self, op: Op) -> Option<NodeID>;
    fn merge(&mut self, ops: Vec<Op>);
    fn nodes(&self) -> Vec<NodeID>;
    fn parent(&self, node: NodeID) -> Option<NodeID>;
    fn children(&self, node: NodeID) -> Vec<TreeNode>;
    fn root(&self) -> NodeID;
    fn get_node(&self, node: NodeID) -> Option<TreeNode>;
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

    pub fn mov(&mut self, target: NodeID, parent: NodeID) {
        let op = Op {
            id: self.new_id(),
            op: TreeOp::Move { target, parent },
        };
        self.ops.entry(self.peer).or_default().push(op);
        self.algorithm.apply(op);
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

    pub fn to_string(&self) -> String {
        let root = self.algorithm.root();
        let root = self.algorithm.get_node(root).unwrap();
        root.to_string("".to_string(), true)
    }
}
