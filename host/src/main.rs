// TODO: Update the name of the method loaded by the prover. E.g., if the method
// is `multiply`, replace `METHOD_NAME_ELF` with `MULTIPLY_ELF` and replace
// `METHOD_NAME_ID` with `MULTIPLY_ID`
use methods::{METHOD_NAME_ELF, METHOD_NAME_ID};
use risc0_zkvm::{default_prover, serde::from_slice, ExecutorEnv};

use hasher::HasherKeccak;

use reth_primitives::{Receipt, ReceiptWithBloomRef};
use risc0_zkvm::serde::to_vec;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cita_trie::node::Node;
use cita_trie::MemoryDB;
use cita_trie::{PatriciaTrie, Trie};
use reth_primitives::bytes::BytesMut;
use reth_primitives::rpc_utils::rlp::RlpStream;
use reth_rlp::Encodable;
use trie_core::{EncodedTrie, Inputs};

pub fn build_from_receipts(receipts: Vec<Receipt>) -> EncodedTrie {
    let memdb = Arc::new(MemoryDB::new(true));
    let hasher = Arc::new(HasherKeccak::new());

    let mut trie = PatriciaTrie::new(memdb.clone(), hasher.clone());
    let mut key_buf = BytesMut::new();
    let mut value_buf = BytesMut::new();

    for (idx, receipt) in receipts.iter().enumerate() {
        key_buf.clear();
        idx.encode(&mut key_buf);

        value_buf.clear();
        let bloom_receipt = ReceiptWithBloomRef::from(receipt);
        bloom_receipt.encode_inner(&mut value_buf, false);
        trie.insert(key_buf.to_vec(), value_buf.to_vec()).unwrap();
    }

    let trie_vec = Rc::new(RefCell::new(Vec::new()));
    encode_trie_rec(trie.root.clone(), trie_vec.clone());

    trie_vec.borrow_mut().reverse();

    let root = trie.root().unwrap();

    EncodedTrie {
        root,
        trie: Rc::try_unwrap(trie_vec).unwrap().into_inner(),
    }
}

fn encode_trie_rec(root: Node, state: Rc<RefCell<Vec<Vec<usize>>>>) -> usize {
    match root {
        Node::Branch(branch) => {
            let borrow_branch = branch.borrow();

            if borrow_branch.value.is_some() {
                panic!("unexpected branch node with value");
            }

            let mut children = Vec::new();
            for i in 0..16 {
                let child = borrow_branch.children[i].clone();

                let child_idx = encode_trie_rec(child, state.clone());
                children.push(child_idx);
            }

            let mut state_inner = state.borrow_mut();
            state_inner.push(children);
            state_inner.len()
        }
        Node::Leaf(leaf) => {
            let borrow_leaf = leaf.borrow();

            let mut stream = RlpStream::new_list(2);
            stream.append(&borrow_leaf.key.encode_compact());
            stream.append(&borrow_leaf.value);

            let buf = stream.out().to_vec();
            let buf = buf
                .iter()
                .cloned()
                .map(|e| e as usize)
                .collect::<Vec<usize>>();

            let mut state_inner = state.borrow_mut();
            state_inner.push(buf);
            state_inner.len()
        }
        Node::Empty => 0,
        _ => {
            panic!("unexpected node type");
        }
    }
}

fn main() {
    // First, we construct an executor environment
    // let env = ExecutorEnv::builder().build().unwrap();

    let receipts_json = std::fs::read("receipts_full.json").unwrap();
    let receipts: Vec<Receipt> = serde_json::from_slice(receipts_json.as_slice()).unwrap();

    let time = std::time::Instant::now();
    let trie = build_from_receipts(receipts);

    let inputs = Inputs { trie };

    let env = ExecutorEnv::builder()
        .add_input(&to_vec(&inputs).unwrap())
        .build()
        .unwrap();

    // TODO: add guest input to the executor environment using
    // ExecutorEnvBuilder::add_input().
    // To access this method, you'll need to use the alternate construction
    // ExecutorEnv::builder(), which creates an ExecutorEnvBuilder. When you're
    // done adding input, call ExecutorEnvBuilder::build().

    // For example:
    // let env = ExecutorEnv::builder().add_input(&vec).build().unwrap();

    // Obtain the default prover.
    let prover = default_prover();

    // Produce a receipt by proving the specified ELF binary.
    let receipt = prover.prove_elf(env, METHOD_NAME_ELF).unwrap();

    println!("time: {:?}", time.elapsed());
    let hash: [u8; 32] = from_slice(&receipt.journal).expect("Error serializing journal output");

    println!("hash: {:?}", hash);
    // Optional: Verify receipt to confirm that recipients will also be able to
    // verify your receipt
    receipt.verify(METHOD_NAME_ID).unwrap();
}
