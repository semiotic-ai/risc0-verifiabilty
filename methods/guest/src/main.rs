#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental

use risc0_zkvm::guest::env;
use std::mem;

risc0_zkvm::guest::entry!(main);
use tiny_keccak::{Hasher, Keccak};
use trie_core::{Inputs, Node};

const SIZEOF_USIZE: usize = mem::size_of::<usize>();

fn keccak256_tiny(mut hasher: Keccak, bytes: &[u8], output: &mut [u8; 32]) {
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
            keccak256_tiny(hasher.to_owned(), &vec, output);
        }
        Node::Leaf(leaf) => keccak256_tiny(hasher.to_owned(), &leaf, output),
        Node::Empty => {
            panic!("unexpected empty node");
        }
    }
}

pub fn main() {
    let inputs: Inputs = env::read();
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    compute_hash(&inputs.root, &mut hasher, &mut output);
    env::commit(&output);
}
