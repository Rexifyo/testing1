/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use ironfish::keys::Language;
use ironfish::signal_catcher::init_signal_handler;
use ironfish::transaction::batch_verify_transactions;
use ironfish::IncomingViewKey;
use ironfish::MerkleNote;
use ironfish::OutgoingViewKey;
use ironfish::PublicAddress;
use ironfish::SaplingKey;
use ironfish::Transaction;
use ironfish::ViewKey;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadSafeCallContext;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi::JsBuffer;
use napi::JsNumber;
use napi_derive::napi;

use ironfish::mining;
use ironfish::sapling_bls12;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;

pub mod mpc;
pub mod nacl;
pub mod rolling_filter;
pub mod signal_catcher;
pub mod structs;

fn to_napi_err(err: impl Display) -> napi::Error {
    Error::from_reason(err.to_string())
}

// unfortunately napi doesn't support reexport of enums (bip39::Language) so we
// have to recreate if we want type safety. hopefully in the future this will work with napi:
// #[napi]
// pub use bip39::Language as Language;
// https://github.com/napi-rs/napi-rs/issues/1463
#[napi]
pub enum LanguageCode {
    English,
    ChineseSimplified,
    ChineseTraditional,
    French,
    Italian,
    Japanese,
    Korean,
    Spanish,
}
impl From<LanguageCode> for Language {
    fn from(item: LanguageCode) -> Self {
        match item {
            LanguageCode::English => Language::English,
            LanguageCode::ChineseSimplified => Language::ChineseSimplified,
            LanguageCode::ChineseTraditional => Language::ChineseTraditional,
            LanguageCode::French => Language::French,
            LanguageCode::Italian => Language::Italian,
            LanguageCode::Japanese => Language::Japanese,
            LanguageCode::Korean => Language::Korean,
            LanguageCode::Spanish => Language::Spanish,
        }
    }
}

#[napi(object)]
pub struct Key {
    pub spending_key: String,
    pub view_key: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub public_address: String,
}

#[napi]
pub fn generate_key() -> Key {
    let sapling_key = SaplingKey::generate_key();

    Key {
        spending_key: sapling_key.hex_spending_key(),
        view_key: sapling_key.view_key().hex_key(),
        incoming_view_key: sapling_key.incoming_view_key().hex_key(),
        outgoing_view_key: sapling_key.outgoing_view_key().hex_key(),
        public_address: sapling_key.public_address().hex_public_address(),
    }
}

#[napi]
pub fn spending_key_to_words(private_key: String, language_code: LanguageCode) -> Result<String> {
    let key = SaplingKey::from_hex(&private_key).map_err(to_napi_err)?;
    let mnemonic = key.to_words(language_code.into()).map_err(to_napi_err)?;
    Ok(mnemonic.into_phrase())
}

#[napi]
pub fn words_to_spending_key(words: String, language_code: LanguageCode) -> Result<String> {
    let key = SaplingKey::from_words(words, language_code.into()).map_err(to_napi_err)?;
    Ok(key.hex_spending_key())
}

#[napi]
pub fn generate_key_from_private_key(private_key: String) -> Result<Key> {
    let sapling_key = SaplingKey::from_hex(&private_key).map_err(to_napi_err)?;

    Ok(Key {
        spending_key: sapling_key.hex_spending_key(),
        view_key: sapling_key.view_key().hex_key(),
        incoming_view_key: sapling_key.incoming_view_key().hex_key(),
        outgoing_view_key: sapling_key.outgoing_view_key().hex_key(),
        public_address: sapling_key.public_address().hex_public_address(),
    })
}

#[napi]
pub fn initialize_sapling() {
    let _ = sapling_bls12::SAPLING.clone();
}

#[napi(constructor)]
pub struct FoundBlockResult {
    pub randomness: String,
    pub mining_request_id: f64,
}

#[napi]
pub struct ThreadPoolHandler {
    threadpool: mining::threadpool::ThreadPool,
}
#[napi]
impl ThreadPoolHandler {
    #[napi(constructor)]
    pub fn new(thread_count: u32, batch_size: u32, pause_on_success: bool) -> Self {
        ThreadPoolHandler {
            threadpool: mining::threadpool::ThreadPool::new(
                thread_count as usize,
                batch_size,
                pause_on_success,
            ),
        }
    }

    #[napi]
    pub fn new_work(&mut self, header_bytes: Buffer, target: Buffer, mining_request_id: u32) {
        self.threadpool
            .new_work(&header_bytes, &target, mining_request_id)
    }

    #[napi]
    pub fn stop(&self) {
        self.threadpool.stop()
    }

    #[napi]
    pub fn pause(&self) {
        self.threadpool.pause()
    }

    #[napi]
    pub fn get_found_block(&self) -> Option<FoundBlockResult> {
        if let Some(result) = self.threadpool.get_found_block() {
            return Some(FoundBlockResult {
                randomness: format!("{:016x}", result.0),
                mining_request_id: result.1 as f64,
            });
        }
        None
    }

    #[napi]
    pub fn get_hash_rate_submission(&self) -> u32 {
        self.threadpool.get_hash_rate_submission()
    }
}

#[napi]
pub fn is_valid_public_address(hex_address: String) -> bool {
    PublicAddress::from_hex(&hex_address).is_ok()
}

// #[napi(object)]
// pub struct NativeDecryptNoteOptions {
//     pub serialized_note: Buffer,
//     pub incoming_view_key: String,
//     pub outgoing_view_key: String,
//     pub view_key: String,
//     pub current_note_index: Option<u32>,
//     pub decrypt_for_spender: bool,
// }

#[napi(object)]
pub struct NativeDecryptNote {
    // pub serialized_note: Vec<u8>,
    // TODO: Stack trace is pointing to Buffer being dropped - this may need ot be a ref or a Vec
    pub serialized_note: Buffer,
    // TODO: Issue is likely related to this being undefined vs null
    pub current_note_index: Option<u32>,
}

#[napi(object)]
pub struct NativeDecryptNoteOptions {
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub view_key: String,
    pub decrypt_for_spender: bool,
    pub notes: Vec<NativeDecryptNote>,
}

#[napi(object, js_name = "NativeDecryptedNote")]
pub struct DecryptedNote {
    pub index: Option<u32>,
    pub for_spender: bool,
    // pub hash: Buffer,
    // pub nullifier: Option<Buffer>,
    // pub serialized_note: Buffer,
    pub hash: Vec<u8>,
    pub nullifier: Option<Vec<u8>>,
    pub serialized_note: Vec<u8>,
}

#[napi]
pub struct NativeWorkerPool {
    pool: ThreadPool,
}

#[napi]
impl NativeWorkerPool {
    #[napi(constructor)]
    pub fn new(size: u32) -> Self {
        Self {
            pool: ThreadPoolBuilder::new()
                .num_threads(size as usize)
                .build()
                .unwrap(),
        }
    }

    #[napi]
    pub fn sleep(&self, callback: JsFunction, ms: u32) -> napi::Result<()> {
        let tscb: ThreadsafeFunction<u32, ErrorStrategy::CalleeHandled> = callback
            .create_threadsafe_function(0, |ctx| {
                // TODO: Why does this need a vec?
                // - The NAPI function returns [error, return value]
                // Can probably change that behavior by changing ErrorStrategy
                ctx.env.create_uint32(ctx.value).map(|v| vec![v])
            })?;

        self.pool.spawn(move || {
            thread::sleep(Duration::from_millis(ms as u64));
            println!("\nRust Done: {:?}\n", ms);
            tscb.call(Ok(ms), ThreadsafeFunctionCallMode::Blocking);
        });

        Ok(())
    }

    #[napi]
    pub fn decrypt_notes(
        &self,
        callback: JsFunction,
        decrypt_note_payload: NativeDecryptNoteOptions,
    ) -> napi::Result<()> {
        let tscb: ThreadsafeFunction<Vec<DecryptedNote>, ErrorStrategy::CalleeHandled> =
            callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

        self.pool.spawn(move || {
            unsafe {
                init_signal_handler();
            }
            // let start = Instant::now();

            // println!("r1");
            let mut incoming_view_key: Option<IncomingViewKey> = None;
            let mut outgoing_view_key: Option<OutgoingViewKey> = None;
            let mut view_key: Option<ViewKey> = None;
            // println!("r2");

            let mut decrypted_notes: Vec<DecryptedNote> = vec![];
            // let mut decrypted_notes: Vec<Option<DecryptedNote>> =
            //     Vec::with_capacity(decrypt_note_payload.notes.len());

            // println!("r3");
            for encrypted_note in decrypt_note_payload.notes {
                // println!("r4");
                let merkle_note =
                    // MerkleNote::read(encrypted_note.serialized_note.as_ref()).unwrap();
                    MerkleNote::read(&encrypted_note.serialized_note[..]).unwrap();
                // MerkleNote::read(&encrypted_note.serialized_note[..]).unwrap();
                // println!("r5");

                let ivk = incoming_view_key.get_or_insert_with(|| {
                    IncomingViewKey::from_hex(&decrypt_note_payload.incoming_view_key).unwrap()
                });
                // println!("r6");

                // let incoming_view_key =
                //     IncomingViewKey::from_hex(&encrypted_note.incoming_view_key).unwrap();

                if let Ok(decrypted_note) = merkle_note.decrypt_note_for_owner(ivk) {
                    // println!("r7");
                    if decrypted_note.value() != 0 {
                        // println!("r8");
                        let nullifier = encrypted_note
                            .current_note_index
                            .filter(|note_index| *note_index != 0) // Hack to match the bug in decryptNotes.ts
                            .map(|note_index| {
                                // let view_key = ViewKey::from_hex(&encrypted_note.view_key).unwrap();

                                // println!("r9");

                                let vk = view_key.get_or_insert_with(|| {
                                    ViewKey::from_hex(&decrypt_note_payload.view_key).unwrap()
                                });
                                // println!("r10");
                                // Buffer::from(&decrypted_note.nullifier(vk, note_index as u64).0[..])
                                decrypted_note.nullifier(vk, note_index as u64).0.into()
                                // decrypted_note
                                //     .nullifier(&view_key, note_index as u64)
                                //     .0
                                //     .as_ref()
                                //     .into()
                            });

                        // let nullifier = if encrypted_note.current_note_index.is_some() {
                        //     let note_index = encrypted_note.current_note_index.unwrap() as u64;
                        //     Some(
                        //         decrypted_note
                        //             .nullifier(&view_key, note_index)
                        //             .0
                        //             .as_ref()
                        //             .into(),
                        //     )
                        // } else {
                        //     None
                        // };

                        // TODO: We can with_capacity this
                        // let mut serialized_note: Vec<u8> = vec![];
                        // println!("r11");
                        let mut serialized_note: Vec<u8> = Vec::with_capacity(168);
                        decrypted_note.write(&mut serialized_note).unwrap();
                        // println!("r12");

                        decrypted_notes.push(DecryptedNote {
                            index: encrypted_note.current_note_index,
                            for_spender: false,
                            // hash: Buffer::from(&merkle_note.merkle_hash().0.to_bytes_le()[..]),
                            // nullifier,
                            hash: merkle_note.merkle_hash().0.to_bytes_le().into(),
                            nullifier,
                            serialized_note: serialized_note.into(),
                        });
                        // println!("r13 -- {:?}", encrypted_note.current_note_index);

                        continue;
                    }
                }

                if decrypt_note_payload.decrypt_for_spender {
                    // println!("r14");
                    // let outgoing_view_key =
                    //     OutgoingViewKey::from_hex(&encrypted_note.outgoing_view_key).unwrap();jA
                    let ovk = outgoing_view_key.get_or_insert_with(|| {
                        OutgoingViewKey::from_hex(&decrypt_note_payload.outgoing_view_key).unwrap()
                    });

                    if let Ok(decrypted_note) = merkle_note.decrypt_note_for_spender(ovk) {
                        if decrypted_note.value() != 0 {
                            // TODO: We can with_capacity this
                            let mut serialized_note: Vec<u8> = Vec::with_capacity(168);
                            decrypted_note.write(&mut serialized_note).unwrap();

                            decrypted_notes.push(DecryptedNote {
                                index: encrypted_note.current_note_index,
                                for_spender: true,
                                hash: merkle_note.merkle_hash().0.to_bytes_le().into(),
                                // hash: Buffer::from(&merkle_note.merkle_hash().0.to_bytes_le()[..]),
                                nullifier: None,
                                serialized_note: serialized_note.into(),
                            });
                            continue;
                        }
                    }
                }

                // decrypted_notes.push(None);
            }

            // println!("r15");
            // let dur = start.elapsed();
            // println!("\nActual Logic (RS): {}\n", dur.as_micros());

            // TODO: Figure out which to use, NonBlocking or Blocking
            tscb.call(Ok(decrypted_notes), ThreadsafeFunctionCallMode::Blocking);
        });

        Ok(())
    }

    #[napi]
    pub fn verify_transactions(
        &self,
        callback: JsFunction,
        // TODO: Does buffer still leak in this way? JsBuffer isn't Send, so figure this out if so.
        serialized_transactions: Vec<Buffer>,
    ) -> napi::Result<()> {
        let tscb: ThreadsafeFunction<bool, ErrorStrategy::CalleeHandled> =
            callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

        self.pool.spawn(move || {
            let mut transactions: Vec<Transaction> = vec![];

            for tx_bytes in serialized_transactions {
                // let buf = tx_bytes.into_value().unwrap(); // TODO:
                // match Transaction::read(buf.as_ref()) {
                match Transaction::read(tx_bytes.as_ref()) {
                    Ok(tx) => transactions.push(tx),
                    Err(_) => {
                        tscb.call(Ok(false), ThreadsafeFunctionCallMode::Blocking);
                        return;
                    }
                }
            }

            let batch_result = batch_verify_transactions(transactions.iter()).is_ok();
            tscb.call(Ok(batch_result), ThreadsafeFunctionCallMode::Blocking);
        });

        Ok(())
    }
}
