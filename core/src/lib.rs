use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncodedTrie {
    pub root: Vec<u8>,
    pub trie: Vec<Vec<usize>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Inputs {
    pub root: Node,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub enum Node {
    Branch {
        children_count: u8,
        children: [Box<Node>; 16],
    },
    Leaf(Vec<u8>),
    #[default]
    Empty,
}
