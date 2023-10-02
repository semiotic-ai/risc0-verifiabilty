#![no_main]
use risc0_zkvm::guest::env;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;

risc0_zkvm::guest::entry!(main);

use tiny_keccak::{Hasher, Keccak};
use trie_core::{build_product_tree, BinaryTree, Inputs, Node, Outputs};

const SIZEOF_USIZE: usize = mem::size_of::<usize>();

fn keccak256_tiny(bytes: &[u8], mut hasher: Keccak, output: &mut [u8; 32]) {
    hasher.update(bytes);
    hasher.finalize(output);
}

pub const fn length_of_length(payload_length: usize) -> usize {
    if payload_length < 56 {
        1
    } else {
        SIZEOF_USIZE - payload_length.leading_zeros() as usize / 8
    }
}

fn compute_hash(node: &Node, hasher: &mut Keccak, output: &mut [u8; 32]) {
    match node {
        Node::Branch {
            children,
            children_count,
        } => {
            let non_empty_children = *children_count as usize;

            let payload_size = non_empty_children * 33 + 17 - non_empty_children;
            let payload_len_len = length_of_length(payload_size);

            let payload_len_bytes = payload_size.to_be_bytes();
            let payload_len_bytes = &payload_len_bytes[payload_len_bytes.len() - payload_len_len..];

            let vec_size = 1 + payload_len_len + payload_size;
            let mut vec = Vec::with_capacity(vec_size);
            vec.push(0xf7 + payload_len_len as u8); // TODO: handle node size < 55 bytes
            vec.extend_from_slice(payload_len_bytes);

            for i in 0..16 {
                let child = children[i].as_ref();
                match child {
                    Node::Empty => {
                        vec.push(128);
                    }
                    _ => {
                        compute_hash(child, hasher, output);
                        vec.push(160); // 128 + 32
                        vec.extend_from_slice(output);
                    }
                }
            }

            vec.push(128); // Empty data
            keccak256_tiny(&vec, hasher.to_owned(), output);
        }
        Node::Leaf(leaf) => keccak256_tiny(&leaf, hasher.to_owned(), output),
        Node::Empty => {
            panic!("unexpected empty node");
        }
    }
}

fn commit(node: &BinaryTree, hasher: &mut Keccak, output: &mut [u8; 32]) {
    match node {
        BinaryTree::Leaf { value } => {
            keccak256_tiny(&value.to_be_bytes(), hasher.to_owned(), output)
        }
        BinaryTree::Branch { left, right, .. } => {
            let mut bytes = [0u8; 2 * 32 + 16];
            commit(left.borrow().deref(), hasher, output);
            bytes[..32].copy_from_slice(output);

            commit(right.borrow().deref(), hasher, output);
            bytes[32..64].copy_from_slice(output);

            bytes[64..].copy_from_slice(&node.value().to_be_bytes());
            keccak256_tiny(bytes.as_slice(), hasher.to_owned(), output)
        }
    }
}

fn build_product_tree_commitment(
    log_addresses: Vec<String>,
    contract_prime: &HashMap<&str, u128>,
    output: &mut [u8; 32],
    hasher: &mut Keccak,
) -> u128 {
    let mut leaves = Vec::new();
    for addr in log_addresses {
        let prime = contract_prime.get(addr.as_str()).unwrap_or(&1);
        leaves.push(Rc::new(RefCell::new(BinaryTree::Leaf { value: *prime })));
    }

    let next_p_2 = leaves.len().next_power_of_two();
    for _ in leaves.len()..next_p_2 {
        leaves.push(Rc::new(RefCell::new(BinaryTree::Leaf { value: 1 })));
    }

    let tree = build_product_tree(leaves);
    let tree = tree.borrow();
    let root_value = tree.value();
    commit(tree.deref(), hasher, output);

    root_value
}

pub fn main() {
    let inputs: Inputs = env::read();
    let mut hasher = Keccak::v256();

    let mut root = [0u8; 32];
    compute_hash(&inputs.root, &mut hasher, &mut root);

    let mut contract_prime = HashMap::new();
    // contract_prime.insert("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984", 2);
    // contract_prime.insert("0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852", 3);
    // contract_prime.insert("0x7a250d5630b4cf539739df2c5dacb4x659f2488d", 5);
    // contract_prime.insert("0x88r6a0c2ddd26feeb64f039a2c41296fcb3f5640", 7);

    contract_prime.insert("0x4ce5df9033ead87976255a8695592bca3e8cb5cb", 2); // Real Values should go here
    contract_prime.insert("0xf64e49c1d1d2b1cfa570b1da6481dc8dc95cd093", 3);
    contract_prime.insert("0x076a3e1500f3110d8f4445d396a3d7ca6d0ca269", 5);

    let mut product_tree_hash = [0u8; 32];
    let product_tree_root = build_product_tree_commitment(
        inputs.log_addresses,
        &contract_prime,
        &mut product_tree_hash,
        &mut hasher,
    );

    let product_tree_root = product_tree_root.to_be_bytes();

    let outputs = Outputs {
        root,
        product_tree_root,
        product_tree_hash,
    };

    println!("outputs: {:?}", outputs);

    env::commit(&outputs);
}
