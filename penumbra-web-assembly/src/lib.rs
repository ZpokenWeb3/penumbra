mod utils;

use std::convert::TryFrom;
use std::fmt::Debug;
use std::str::FromStr;
use penumbra_crypto::{FullViewingKey, IdentityKey, ka, Note, NotePayload, Nullifier};
use anyhow::Context;
use anyhow::Error;
use hex::FromHex;


use wasm_bindgen::prelude::*;
use penumbra_crypto::keys::{SeedPhrase, SpendKey};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}
#[wasm_bindgen]
pub fn decrypt_note(full_viewing_key: &str, encrypted_note: &str, ephemeral_key: &str) -> JsValue {
    utils::set_panic_hook();
    let fvk = FullViewingKey::from_str(full_viewing_key.as_ref())
        .context("The provided string is not a valid FullViewingKey");

    let note = Note::decrypt(&hex::decode(encrypted_note).unwrap()[..],
                             fvk.unwrap().incoming(),
                             &ka::Public::try_from(&hex::decode(ephemeral_key).unwrap()[..]).unwrap())
        .unwrap();


    return  JsValue::from_serde(&note).unwrap();
}

#[wasm_bindgen]
pub fn generate_spend_key(seed_phrase: &str) -> JsValue {
    let seed = SeedPhrase::from_str(seed_phrase).unwrap();
    let spend_key = SpendKey::from_seed_phrase(seed, 0);

    return  JsValue::from_serde(&spend_key).unwrap();
}

#[wasm_bindgen]
pub fn get_full_viewing_key(spend_key_str: &str) -> JsValue {
    let spend_key = SpendKey::from_str(spend_key_str).unwrap();

    return  JsValue::from_serde(&spend_key.full_viewing_key()).unwrap();
}


