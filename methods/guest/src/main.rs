#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental

use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);
use tiny_keccak::{Hasher, Keccak};
use trie_core::{Inputs, Node};

fn keccak256_tiny(mut hasher: Keccak, bytes: &[u8], output: &mut [u8; 32]) {
    hasher.update(bytes);
    hasher.finalize(output);
}

pub const fn length_of_length(payload_length: usize) -> usize {
    if payload_length < 56 {
        1
    } else {
        4 - payload_length.leading_zeros() as usize / 8
    }
}

// fn compute_root(trie: &Vec<Vec<usize>>, idx: usize) -> Option<[u8; 32]> {
//     if idx >= trie.len() {
//         return None;
//     }
//
//     let node = &trie[idx];
//     return if node.len() == 16 {
//         let non_empty_children = node.iter().filter(|e| **e != 0).count();
//         let payload_size = non_empty_children * 33 + 17 - non_empty_children;
//         let payload_len_len = length_of_length(payload_size);
//
//         let payload_len_bytes = payload_size.to_be_bytes();
//         let payload_len_bytes = &payload_len_bytes[payload_len_bytes.len() - payload_len_len..];
//
//         let mut vec = vec![0xf7 + payload_len_len as u8];
//         vec.extend_from_slice(payload_len_bytes);
//
//         for i in 0..16 {
//             let child_idx = node[i];
//             if child_idx == 0 {
//                 vec.push(0x80);
//                 continue;
//             }
//
//             let child_hash = compute_root(trie, child_idx);
//             if let Some(data) = child_hash {
//                 vec.push(160);
//                 vec.extend_from_slice(&data);
//             }
//         }
//
//         vec.push(128); // Empty data
//         let hash = keccak256_tiny(&vec);
//         Some(hash.try_into().unwrap())
//     } else {
//         // let buf = node.iter().cloned().map(|e| e as u8).collect::<Vec<u8>>();
//         let buf: Vec<u8> = node.iter().cloned().map(|e| e as u8).collect();
//         let hash = keccak256_tiny(&buf);
//         Some(hash.try_into().unwrap())
//     };
// }

fn compute_hash(node: &Node, hasher: Keccak, output: &mut [u8; 32]) {
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
            // let mut vec = vec![0xf7 + payload_len_len as u8];
            vec.push(0xf7 + payload_len_len as u8);
            vec.extend_from_slice(payload_len_bytes);

            for i in 0..16 {
                let child = &children[i];
                let child = child.as_ref();
                match child {
                    Node::Empty => {
                        vec.push(128);
                    }
                    _ => {
                        compute_hash(child, hasher.clone(), output);
                        vec.push(160);
                        vec.extend_from_slice(output);
                    }
                }
            }

            vec.push(128); // Empty data
            keccak256_tiny(hasher, &vec, output);
        }
        Node::Leaf(leaf) => keccak256_tiny(hasher, &leaf, output),
        Node::Empty => {
            panic!("unexpected empty node");
        }
    }
}

pub fn main() {
    let inputs: Inputs = env::read();
    let mut output = [0u8; 32];
    let hasher = Keccak::v256();
    compute_hash(&inputs.root, hasher, &mut output);
    env::commit(&output);
}
