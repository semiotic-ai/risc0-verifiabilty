#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental

use risc0_zkvm::guest::env;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;

risc0_zkvm::guest::entry!(main);
use tiny_keccak::{Hasher, Keccak};
use trie_core::{build_product_tree, BinaryTree, Inputs, Node};

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
) {
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
    let hasher = Keccak::v256();
    commit(tree.borrow().deref(), &mut hasher.clone(), output);
}

pub fn main() {
    let inputs: Inputs = env::read();
    let mut receipts_root = [0u8; 32];
    let mut hasher = Keccak::v256();

    let mut contract_prime = HashMap::new();
    // contract_prime.insert("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", 2);
    // contract_prime.insert("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", 3);
    // contract_prime.insert("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", 5);
    contract_prime.insert("0x4ce5df9033ead87976255a8695592bca3e8cb5cb", 2);
    contract_prime.insert("0xf64e49c1d1d2b1cfa570b1da6481dc8dc95cd093", 3);
    contract_prime.insert("0x076a3e1500f3110d8f4445d396a3d7ca6d0ca269", 5);

    compute_hash(&inputs.root, &mut hasher, &mut receipts_root);
    let mut product_tree_root = [0u8; 32];
    build_product_tree_commitment(
        inputs.log_addresses,
        &contract_prime,
        &mut product_tree_root,
    );
    env::commit(&receipts_root);
    env::commit(&product_tree_root);
}
