use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Inputs {
    pub root: Node,
    pub log_addresses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Outputs {
    pub root: [u8; 32],
    pub product_tree_hash: [u8; 32],
    pub product_tree_root: [u8; 16], // 16 bytes for u128
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

pub enum BinaryTree {
    Leaf {
        value: u128,
    },
    Branch {
        left: TreeNodeRef,
        right: TreeNodeRef,
        value: u128,
    },
}

type TreeNodeRef = Rc<RefCell<BinaryTree>>;

impl BinaryTree {
    pub fn value(&self) -> u128 {
        match self {
            BinaryTree::Leaf { value } => *value,
            BinaryTree::Branch { value, .. } => *value,
        }
    }

    pub fn collect(&self) -> Vec<u128> {
        match self {
            BinaryTree::Leaf { value } => vec![*value],
            BinaryTree::Branch { left, right, .. } => {
                let mut left = left.borrow().collect();
                let mut right = right.borrow().collect();

                left.append(&mut right);
                left.push(self.value());

                left
            }
        }
    }
}

pub fn map_leaves(leaves: Vec<u128>) -> Vec<Rc<RefCell<BinaryTree>>> {
    let mut leaves: Vec<Rc<RefCell<BinaryTree>>> = leaves
        .iter()
        .map(|leaf| Rc::new(RefCell::new(BinaryTree::Leaf { value: *leaf })))
        .collect();

    let next_p_2 = leaves.len().next_power_of_two();

    for _ in leaves.len()..next_p_2 {
        leaves.push(Rc::new(RefCell::new(BinaryTree::Leaf { value: 1 })));
    }

    leaves
}

/**
 * Build a product tree from a list of leaves.
 * Leaves len must be a power of 2. Complete with 1s if not.
 * The product tree is a binary tree where each node is the product of its children.
 **/
pub fn build_product_tree(leaves: Vec<Rc<RefCell<BinaryTree>>>) -> Rc<RefCell<BinaryTree>> {
    if leaves.len() == 1 {
        return leaves[0].clone();
    }

    let mut branches = Vec::new();
    for i in (0..leaves.len()).step_by(2) {
        let left = leaves[i].clone();
        let right = leaves[i + 1].clone();

        let value = left.borrow().value() * right.borrow().value();

        let branch = BinaryTree::Branch { value, left, right };

        branches.push(Rc::new(RefCell::new(branch)));
    }

    build_product_tree(branches)
}

pub fn factor_of_n(val: u128, n: u8) -> u8 {
    let mut count = 0;
    let mut val = val;
    while val % n as u128 == 0 {
        count += 1;
        val /= n as u128;
    }

    count
}
