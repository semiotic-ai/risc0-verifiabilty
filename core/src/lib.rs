use serde::{Deserialize, Serialize};

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
