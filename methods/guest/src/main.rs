#![no_main]
// If you want to try std support, also update the guest Cargo.toml file
// #![no_std]  // std support is experimental

use risc0_zkvm::guest::env;
use std::mem::size_of;

risc0_zkvm::guest::entry!(main);
use tiny_keccak::{Hasher, Keccak};
use trie_core::Inputs;

const USIZE_SIZE: usize = size_of::<usize>();

fn keccak256_tiny(mut hasher: Keccak, bytes: &[u8], output: &mut [u8; 32]) {
    hasher.update(bytes);
    hasher.finalize(output);
}

pub const fn length_of_length(payload_length: usize) -> usize {
    if payload_length < 56 {
        1
    } else {
        USIZE_SIZE - payload_length.leading_zeros() as usize / 8
    }
}

fn compute_root(trie: &Vec<Vec<usize>>, idx: usize, output: &mut [u8; 32]) {
    let hasher = Keccak::v256();
    let node = &trie[idx];
    return if node.len() == 16 {
        let non_empty_children = node.iter().filter(|e| **e != 0).count();
        let payload_size = non_empty_children * 33 + 17 - non_empty_children;
        let payload_len_len = length_of_length(payload_size);

        let payload_len_bytes = payload_size.to_be_bytes();
        let payload_len_bytes = &payload_len_bytes[payload_len_bytes.len() - payload_len_len..];

        let mut vec = Vec::with_capacity(1 + payload_len_len + payload_size);
        vec.push(0xf7 + payload_len_len as u8);
        vec.extend_from_slice(payload_len_bytes);

        for i in 0..16 {
            let child_idx = node[i];
            if child_idx == 0 {
                vec.push(0x80);
            } else if child_idx <= trie.len() {
                compute_root(trie, child_idx - 1, output);
                vec.push(160); // 128 + 32
                vec.extend_from_slice(output);
            }
        }

        vec.push(0x80); // Empty data
        keccak256_tiny(hasher, &vec, output);
    } else {
        let buf: Vec<u8> = node.iter().map(|e| *e as u8).collect();
        keccak256_tiny(hasher, &buf, output);
    };
}

pub fn main() {
    let inputs: Inputs = env::read();
    let mut output = [0u8; 32];
    compute_root(&inputs.trie.trie, inputs.trie.trie.len() - 1, &mut output);
    env::commit(&output);
}
