use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EncodedTrie {
    pub root: Vec<u8>,
    pub trie: Vec<Vec<usize>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Inputs {
    pub trie: EncodedTrie,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Node {
    Branch([Box<Node>; 16]),
    Leaf(Vec<u8>),
    Empty,
}
