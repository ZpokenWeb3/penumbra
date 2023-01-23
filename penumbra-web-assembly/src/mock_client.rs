use anyhow::Context;
use penumbra_chain::{CompactBlock, Epoch, StatePayload};
use penumbra_crypto::Nullifier;
use penumbra_crypto::{dex::swap::SwapPlaintext, note, FullViewingKey, Note};
use penumbra_tct as tct;
use penumbra_tct::Witness::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::{collections::BTreeMap, str::FromStr};
use tct::storage::{StoredPosition, Updates};
use tct::structure::Hash;
use tct::{Forgotten, Position, Tree};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console as web_console;

use crate::note_record::SpendableNoteRecord;
use crate::swap_record::SwapRecord;
use crate::utils;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTree {
    last_position: u64,
    last_forgotten: u64,
    hashes: Vec<StoredHash>,
    commitments: Vec<StoredCommitment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredHash {
    position: u64,
    height: u8,
    hash: [u8; 32],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredCommitment {
    position: u64,
    commitment: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanBlockResult {
    height: u64,
    nct_updates: Updates,
    new_notes: Vec<SpendableNoteRecord>,
    new_swaps: Vec<SwapRecord>,
}



impl ScanBlockResult {
    pub fn new(
        height: u64,
        nct_updates: Updates,
        new_notes: Vec<SpendableNoteRecord>,
        new_swaps: Vec<SwapRecord>,
    ) -> ScanBlockResult {
        Self {
            height,
            nct_updates,
            new_notes,
            new_swaps,
        }
    }
}

#[wasm_bindgen]
pub struct ViewClient {
    latest_height: u64,
    epoch_duration: u64,
    fvk: FullViewingKey,
    notes: BTreeMap<note::Commitment, Note>,
    swaps: BTreeMap<tct::Commitment, SwapPlaintext>,
    nct: penumbra_tct::Tree,
}

#[wasm_bindgen]
impl ViewClient {
    #[wasm_bindgen(constructor)]
    pub fn new(full_viewing_key: &str, epoch_duration: u64, stored_tree: JsValue) -> ViewClient {
        utils::set_panic_hook();
        let fvk = FullViewingKey::from_str(full_viewing_key.as_ref())
            .context("The provided string is not a valid FullViewingKey")
            .unwrap();

        let stored_tree: StoredTree = serde_wasm_bindgen::from_value(stored_tree).unwrap();

        let position: Position = stored_tree.last_position.try_into().unwrap();
        let position_option: Option<Position> = Some(position);
        let stored_position: StoredPosition = position_option.try_into().unwrap();

        let mut add_commitments = Tree::load(
            stored_position,
            stored_tree.last_forgotten.try_into().unwrap(),
        );

        for store_commitment in &stored_tree.commitments {
            add_commitments.insert(
                store_commitment.position.try_into().unwrap(),
                store_commitment.commitment.try_into().unwrap(),
            )
        }
        let mut add_hashes = add_commitments.load_hashes();

        for stored_hash in &stored_tree.hashes {
            add_hashes.insert(
                stored_hash.position.try_into().unwrap(),
                stored_hash.height,
                Hash::from_bytes(stored_hash.hash).unwrap(),
            );
        }
        let tree = add_hashes.finish();

        Self {
            latest_height: u64::MAX,
            fvk,
            epoch_duration,
            notes: Default::default(),
            nct: tree,
            swaps: Default::default(),
        }
    }

    #[wasm_bindgen]
    pub fn scan_block(
        &mut self,
        compact_block: JsValue,
        last_position: u64,
        last_forgotten: u64,
    ) -> JsValue {
        utils::set_panic_hook();
        web_console::log_1(&"Start scan_block()".into());
        web_console::log_1(&compact_block);

        let position: Position = last_position.try_into().unwrap();
        let position_option: Option<Position> = Some(position);
        let stored_position: StoredPosition = position_option.try_into().unwrap();

        web_console::log_1(&"StoredPosition is loaded".into());


        let block_proto: penumbra_proto::core::chain::v1alpha1::CompactBlock =
            serde_wasm_bindgen::from_value(compact_block).unwrap();

        let block: CompactBlock = block_proto.try_into().unwrap();

        web_console::log_1(&"Success deserileze".into());

        // Trial-decrypt the notes in this block, keeping track of the ones that were meant for us

        // Newly detected spendable notes.
        let mut new_notes = Vec::new();
        // Newly detected claimable swaps.
        let mut new_swaps: Vec<SwapRecord> = Vec::new();

//        if self.latest_height.wrapping_add(1) != block.height {
//            return Default::default();
//        }

        for state_payload in block.state_payloads {
            let clone_payload = state_payload.clone();
            web_console::log_1(&"Handle state_payloads".into());

            match state_payload {
                StatePayload::Note { note: payload, .. } => {
                    match payload.trial_decrypt(&self.fvk) {
                        Some(note) => {
                            self.notes.insert(payload.note_commitment, note.clone());
                            let note_position =
                                self.nct.insert(Keep, payload.note_commitment).unwrap();

                            let source = clone_payload.source().cloned().unwrap_or_default();
                            let nullifier = self
                                .fvk
                                .derive_nullifier(position, clone_payload.commitment());
                            let address_index = self
                                .fvk
                                .incoming()
                                .index_for_diversifier(note.diversifier());


                            web_console::log_1(&"Found new notes".into());

                            new_notes.push(SpendableNoteRecord {
                                note_commitment: clone_payload.commitment().clone(),
                                height_spent: None,
                                height_created: block.height,
                                note: note.clone(),
                                address_index,
                                nullifier,
                                position: note_position,
                                source,
                            });
                        }
                        None => {
                            self.nct.insert(Forget, payload.note_commitment).unwrap();
                        }
                    }
                }
                StatePayload::Swap { swap: payload, .. } => {
                    match payload.trial_decrypt(&self.fvk) {
                        Some(swap) => {
                            let swap_position = self.nct.insert(Keep, payload.commitment).unwrap();
                            // At this point, we need to retain the swap plaintext,
                            // and also derive the expected output notes so we can
                            // notice them while scanning later blocks.
                            self.swaps.insert(payload.commitment, swap.clone());

                            let batch_data = block
                                .swap_outputs
                                .get(&swap.trading_pair)
                                .ok_or_else(|| anyhow::anyhow!("server gave invalid compact block"))
                                .unwrap();

                            let (output_1, output_2) = swap.output_notes(batch_data);
                            // Pre-insert the output notes into our notes table, so that
                            // we can notice them when we scan the block where they are claimed.
                            self.notes.insert(output_1.commit(), output_1);
                            self.notes.insert(output_2.commit(), output_2);

                            let source = clone_payload.source().cloned().unwrap_or_default();
                            let nullifier = self
                                .fvk
                                .derive_nullifier(position, clone_payload.commitment());

                            new_swaps.push(SwapRecord {
                                swap_commitment: clone_payload.commitment().clone(),
                                swap: swap.clone(),
                                position: swap_position,
                                nullifier,
                                source,
                                output_data: batch_data.clone(),
                                height_claimed: None,
                            });
                        }
                        None => {
                            self.nct.insert(Forget, payload.commitment).unwrap();
                        }
                    }
                }
                StatePayload::RolledUp(commitment) => {
                    if self.notes.contains_key(&commitment) {
                        // This is a note we anticipated, so retain its auth path.
                        self.nct.insert(Keep, commitment).unwrap();
                    } else {
                        // This is someone else's note.
                        self.nct.insert(Forget, commitment).unwrap();
                    }
                }
            }
        }

        self.nct.end_block().unwrap();
        if Epoch::from_height(block.height, self.epoch_duration).is_epoch_end(block.height) {
            self.nct.end_epoch().unwrap();
        }

        self.latest_height = block.height;

        let nct_updates: Updates = self
            .nct
            .updates(stored_position, last_forgotten.try_into().unwrap())
            .collect::<Updates>();

        let result = ScanBlockResult {
            height: self.latest_height,
            nct_updates,
            new_notes,
            new_swaps,
        };

        return serde_wasm_bindgen::to_value(&result).unwrap();
    }
}
