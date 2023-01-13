 use std::collections::BTreeMap;
 use penumbra_chain::{CompactBlock, StatePayload, Epoch};
use penumbra_crypto::{dex::swap::SwapPlaintext, note, FullViewingKey, Note};
 use penumbra_tct as tct;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use penumbra_tct::Witness::*;



 /// A bare-bones mock client for use exercising the state machine.
 #[wasm_bindgen]
 pub struct MockClient {
     latest_height: u64,
     epoch_duration: u64,
     fvk: FullViewingKey,
     notes: BTreeMap<note::Commitment, Note>,
     swaps: BTreeMap<tct::Commitment, SwapPlaintext>,
     nct: penumbra_tct::Tree,
 }


 #[wasm_bindgen]
 impl MockClient {
//     pub fn new(fvk: FullViewingKey, epoch_duration: u64) -> MockClient {
//         Self {
//             latest_height: u64::MAX,
//             fvk,
//             epoch_duration,
//             notes: Default::default(),
//             nct: Default::default(),
//             swaps: Default::default(),
//         }
//     }

     #[wasm_bindgen]
     pub fn scan_block(&mut self, compactBlock: JsValue) -> JsValue {

         let block : CompactBlock = serde_wasm_bindgen::from_value(compactBlock).unwrap();

         if self.latest_height.wrapping_add(1) != block.height {
             return Default::default();
         }


         for payload in block.state_payloads {
             match payload {
                 StatePayload::Note { note: payload, .. } => {
                     match payload.trial_decrypt(&self.fvk) {
                         Some(note) => {
                             self.notes.insert(payload.note_commitment, note.clone());
                             self.nct.insert(Keep, payload.note_commitment).unwrap();
                         }
                         None => {
                             self.nct.insert(Forget, payload.note_commitment).unwrap();
                         }
                     }
                 }
                 StatePayload::Swap { swap: payload, .. } => {
                     match payload.trial_decrypt(&self.fvk) {
                         Some(swap) => {
                             self.nct.insert(Keep, payload.commitment).unwrap();
                             // At this point, we need to retain the swap plaintext,
                             // and also derive the expected output notes so we can
                             // notice them while scanning later blocks.
                             self.swaps.insert(payload.commitment, swap.clone());

                             let batch_data =
                                block.swap_outputs.get(&swap.trading_pair).ok_or_else(|| {
                                    anyhow::anyhow!("server gave invalid compact block")
                                }).unwrap();

                             let (output_1, output_2) = swap.output_notes(batch_data);
                             // Pre-insert the output notes into our notes table, so that
                             // we can notice them when we scan the block where they are claimed.
                             self.notes.insert(output_1.commit(), output_1);
                             self.notes.insert(output_2.commit(), output_2);
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

         return Default::default();
     }

//     pub fn latest_height_and_nct_root(&self) -> (u64, penumbra_tct::Root) {
//         (self.latest_height, self.nct.root())
//     }
//
//     pub fn note_by_commitment(&self, commitment: &note::Commitment) -> Option<Note> {
//         self.notes.get(commitment).cloned()
//     }
//
//     pub fn swap_by_commitment(&self, commitment: &note::Commitment) -> Option<SwapPlaintext> {
//         self.swaps.get(commitment).cloned()
//     }
//
//     pub fn witness(&self, commitment: note::Commitment) -> Option<penumbra_tct::Proof> {
//         self.nct.witness(commitment)
//     }
 }

