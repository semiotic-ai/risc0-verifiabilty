use std::collections::HashMap;
// TODO: Update the name of the method loaded by the prover. E.g., if the method
// is `multiply`, replace `METHOD_NAME_ELF` with `MULTIPLY_ELF` and replace
// `METHOD_NAME_ID` with `MULTIPLY_ID`
use methods::{METHOD_NAME_ELF, METHOD_NAME_ID};
use risc0_zkvm::{default_prover, serde::from_slice, ExecutorEnv};

use hasher::HasherKeccak;

use reth_primitives::{Receipt, ReceiptWithBloomRef};
use risc0_zkvm::serde::to_vec;
use std::sync::Arc;

use cita_trie::MemoryDB;
use cita_trie::{PatriciaTrie, Trie};
use reth_primitives::bytes::BytesMut;
use reth_primitives::rpc_utils::rlp::RlpStream;
use reth_rlp::Encodable;
use trie_core::{Inputs, Node};

pub fn build_from_receipts(receipts: Vec<Receipt>) -> (Vec<String>, Node) {
    let mem_db = Arc::new(MemoryDB::new(true));
    let hasher = Arc::new(HasherKeccak::new());

    let mut trie = PatriciaTrie::new(mem_db.clone(), hasher.clone());
    let mut key_buf = BytesMut::new();
    let mut value_buf = BytesMut::new();

    let mut log_addresses: Vec<String> = Vec::new();

    let mut contract_prime = HashMap::new();
    // contract_prime.insert("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", 2);
    // contract_prime.insert("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", 3);
    // contract_prime.insert("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", 5);
    contract_prime.insert("0x4ce5df9033ead87976255a8695592bca3e8cb5cb", 2);
    contract_prime.insert("0xf64e49c1d1d2b1cfa570b1da6481dc8dc95cd093", 3);
    contract_prime.insert("0x076a3e1500f3110d8f4445d396a3d7ca6d0ca269", 5);

    for (idx, receipt) in receipts.iter().enumerate() {
        key_buf.clear();
        idx.encode(&mut key_buf);

        receipt.logs.iter().for_each(|log| {
            let addr = format!("{:?}", log.address);
            log_addresses.push(addr);
        });

        // println!("leaves: {:?}", product_tree_leaves);

        value_buf.clear();
        let bloom_receipt = ReceiptWithBloomRef::from(receipt);
        bloom_receipt.encode_inner(&mut value_buf, false);
        trie.insert(key_buf.to_vec(), value_buf.to_vec()).unwrap();
    }

    // let leaves = map_leaves(product_tree_leaves);
    // let product_tree = build_product_tree(leaves);
    // println!("value: {}", product_tree.borrow().value());
    //
    // println!("collect: {:?}", product_tree.borrow().collect());
    // println!("commit: {:?}", product_tree.borrow().commit());

    (log_addresses, encode_trie_rec(trie.root))
}

fn encode_trie_rec(root: cita_trie::node::Node) -> Node {
    match root {
        cita_trie::node::Node::Branch(branch) => {
            let borrow_branch = branch.borrow();
            let mut children: [Box<Node>; 16] = Default::default();
            let mut children_count = 0;
            for i in 0..16 {
                let child = borrow_branch.children[i].clone();

                let child = encode_trie_rec(child);
                match child {
                    Node::Empty => {}
                    _ => {
                        children_count += 1;
                    }
                }
                children[i] = Box::new(child);
            }

            Node::Branch {
                children,
                children_count,
            }
        }
        cita_trie::node::Node::Leaf(leaf) => {
            let borrow_leaf = leaf.borrow();

            let mut stream = RlpStream::new_list(2);
            stream.append(&borrow_leaf.key.encode_compact());
            stream.append(&borrow_leaf.value);

            let buf = stream.out().to_vec();
            Node::Leaf(buf)
        }
        cita_trie::node::Node::Empty => Node::Empty,
        _ => {
            panic!("unexpected node type");
        }
    }
}

fn main() {
    let receipts_json = std::fs::read("receipts_full.json").unwrap();
    let receipts: Vec<Receipt> = serde_json::from_slice(receipts_json.as_slice()).unwrap();

    let time = std::time::Instant::now();
    let (log_addresses, root) = build_from_receipts(receipts);

    let inputs = Inputs {
        root,
        log_addresses,
    };

    println!("time: {:?}", time.elapsed());

    let env = ExecutorEnv::builder()
        .add_input(&to_vec(&inputs).unwrap())
        .build()
        .unwrap();

    // Obtain the default prover.
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary.
    let receipt = prover.prove_elf(env, METHOD_NAME_ELF).unwrap();

    println!("time: {:?}", time.elapsed());
    let hash: [u8; 32] = from_slice(&receipt.journal).expect("Error serializing journal output");
    println!("hash: {:?}", hash);

    let product_tree_hash: [u8; 32] =
        from_slice(&receipt.journal[32..]).expect("Error serializing output");
    println!("product_tree_hash: {:?}", product_tree_hash);

    // Optional: Verify receipt to confirm that recipients will also be able to
    // verify your receipt
    receipt.verify(METHOD_NAME_ID).unwrap();
}
