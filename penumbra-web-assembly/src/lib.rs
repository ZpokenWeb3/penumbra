extern crate core;

mod utils;

use penumbra_proto::{Protobuf};
use std::convert::TryFrom;
use std::fmt::Debug;
use std::str::FromStr;
use penumbra_crypto::{FullViewingKey, IdentityKey, ka, Note, NotePayload, Nullifier};
use anyhow::Context;
use anyhow::Error;
use hex::FromHex;
use base64::decode;


use wasm_bindgen::prelude::*;
use penumbra_crypto::keys::{SeedPhrase, SpendKey};
use penumbra_tct::{Forgotten, Tree};
use penumbra_tct::storage::{StoredPosition, Updates};
use penumbra_transaction::Transaction;


#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


#[wasm_bindgen]
pub fn decrypt_note(full_viewing_key: &str, encrypted_note: &str, ephemeral_key: &str) -> JsValue {
    utils::set_panic_hook();
    let fvk = FullViewingKey::from_str(full_viewing_key.as_ref())
        .context("The provided string is not a valid FullViewingKey");

    let note = Note::decrypt(&hex::decode(encrypted_note).unwrap()[..],
                             fvk.unwrap().incoming(),
                             &ka::Public::try_from(&hex::decode(ephemeral_key).unwrap()[..]).unwrap());


    return if note.is_ok() {
        JsValue::from_serde(&note.unwrap()).unwrap()
    } else {
        JsValue::null()
    };
}

#[wasm_bindgen]
pub fn generate_spend_key(seed_phrase: &str) -> JsValue {
    let seed = SeedPhrase::from_str(seed_phrase).unwrap();
    let spend_key = SpendKey::from_seed_phrase(seed, 0);

    return JsValue::from_serde(&spend_key).unwrap();
}

#[wasm_bindgen]
pub fn get_full_viewing_key(spend_key_str: &str) -> JsValue {
    let spend_key = SpendKey::from_str(spend_key_str).unwrap();

    return JsValue::from_serde(&spend_key.full_viewing_key()).unwrap();
}

#[wasm_bindgen]
pub fn get_address_by_index(full_viewing_key: &str, index: u64) -> JsValue {
    let fvk = FullViewingKey::from_str(full_viewing_key.as_ref())
        .context("The provided string is not a valid FullViewingKey").unwrap();

    let (address, _dtk) = fvk
        .incoming()
        .payment_address(index.into());
    return JsValue::from_serde(&address).unwrap();
}

#[wasm_bindgen]
pub fn decode_transaction(tx_bytes: &str) -> JsValue {
    let transaction = Transaction::decode(base64::decode(tx_bytes).unwrap().as_slice()).unwrap();
    return JsValue::from_serde(&transaction).unwrap();
}

#[wasm_bindgen]
pub fn send(full_viewing_key: &str,
            amount: u64,
            asset_id: &str,
            fee: u64,
            dest_address: &str) -> JsValue {
    return JsValue::null();
}


// #[wasm_bindgen]
// pub fn nct_insert_empty_block(stored_position: JsValue,
//                               last_forgotten: JsValue,
//                               height: u64,
//                               epoch_duration: u64) -> Result<JsValue, JsValue> {
//     let position: StoredPosition = serde_wasm_bindgen::from_value(stored_position).unwrap();
//     let forgotten: Forgotten = serde_wasm_bindgen::from_value(last_forgotten).unwrap();
//
//
//     let load_c = Tree::load(position, forgotten);
//     let load_h = load_c.load_hashes();
//     let mut nct = load_h.finish();
//
//     nct.end_block().unwrap();
//     if Epoch::from_height(height, epoch_duration).is_epoch_end(height) {
//         nct
//             .end_epoch()
//             .expect("ending the epoch must succeed");
//     }
//
//     let updates = nct.updates(position, forgotten).collect::<Updates>();
// }

//
//     Ok(serde_wasm_bindgen::to_value(&updates)?)}
//
// pub fn deserialize_nct(stored_position: JsValue,
//                        last_forgotten: JsValue) -> Tree {
//     let position: StoredPosition = serde_wasm_bindgen::from_value(stored_position)?;
//     let forgotten: Forgotten = serde_wasm_bindgen::from_value(last_forgotten)?;
//
//
//     let load_c = Tree::load(position, forgotten);
//     let load_h = load_c.load_hashes();
//     let nct = load_h.finish();
//
//     return nct;
// }
//
//
//
//
