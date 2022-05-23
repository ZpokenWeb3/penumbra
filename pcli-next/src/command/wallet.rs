use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context as _, Result};
use directories::ProjectDirs;
use penumbra_crypto::keys::{SeedPhrase, SpendSeed};
use penumbra_wallet_next::{ClientState, Wallet};
use rand_core::OsRng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use structopt::StructOpt;

use crate::{wallet::Wallet, ClientStateFile};

#[derive(Debug, StructOpt)]
pub enum WalletCmd {
    /// Import from an existing seed phrase.
    ImportFromPhrase {
        /// A 24 word phrase in quotes.
        seed_phrase: String,
    },
    /// Export the full viewing key for the wallet.
    ExportFvk,
    /// Generate a new seed phrase.
    Generate,
    /// Keep the spend seed, but reset all other client state.
    Reset,
    /// Delete the entire wallet permanently.
    Delete,
}

impl WalletCmd {
    /// Determine if this command requires a network sync before it executes.
    pub fn needs_sync(&self) -> bool {
        match self {
            WalletCmd::ImportFromPhrase { .. } => false,
            WalletCmd::ExportFvk => false,
            WalletCmd::Generate => false,
            WalletCmd::Reset => false,
            WalletCmd::Delete => false,
        }
    }

    pub fn save_wallet(&self, data_dir: PathBuf, wallet: Wallet) -> Result<()> {
        todo!()
    }

    pub fn exec(&self, data_dir: PathBuf) -> Result<()> {
        match self {
            WalletCmd::Generate => {
                let seed_phrase = SeedPhrase::generate(&mut OsRng);

                // xxx: Something better should be done here, this is in danger of being
                // shared by users accidentally in log output.
                println!(
                    "YOUR PRIVATE SEED PHRASE: {}\nDO NOT SHARE WITH ANYONE!",
                    seed_phrase
                );

                let wallet = Wallet::from_seed_phrase(seed_phrase);
                self.save_wallet(data_dir, wallet);
            }
            WalletCmd::ImportFromPhrase { seed_phrase } => {
                let wallet = Wallet::from_seed_phrase(SeedPhrase::from_str(seed_phrase)?);
                self.save_wallet(data_dir, wallet);
            }
            // The rest of these commands don't require a wallet state to be saved to disk:
            WalletCmd::ExportFvk => {
                let state = ClientStateFile::load(wallet_path.clone())?;
                println!("{}", state.wallet().full_viewing_key());
            }
            WalletCmd::Delete => {
                if wallet_path.is_file() {
                    std::fs::remove_file(&wallet_path)?;
                    println!("Deleted wallet file at {}", wallet_path.display());
                } else if wallet_path.exists() {
                    return Err(anyhow!(
                            "Expected wallet file at {} but found something that is not a file; refusing to delete it",
                            wallet_path.display()
                        ));
                } else {
                    return Err(anyhow!(
                        "No wallet exists at {}, so it cannot be deleted",
                        wallet_path.display()
                    ));
                }
                None
            }
            WalletCmd::Reset => {
                tracing::info!("resetting client state");

                tracing::debug!("reading existing client state from disk");

                #[derive(Deserialize)]
                struct MinimalState {
                    wallet: Wallet,
                }

                // Read the wallet field out of the state file, without fully deserializing the rest
                let wallet = serde_json::from_reader::<_, MinimalState>(BufReader::new(
                    File::open(&wallet_path)?,
                ))?
                .wallet;

                tracing::debug!("writing fresh client state");

                // Write the new wallet JSON to disk as a temporary file in the wallet directory
                let tmp_path = wallet_path.with_extension("tmp");
                let mut tmp_file = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&tmp_path)?;

                serde_json::to_writer_pretty(&mut tmp_file, &ClientState::new(wallet))?;

                tracing::debug!("checking that we can deserialize fresh client state");

                // Check that we can successfully parse the result from disk
                ClientStateFile::load(tmp_path.clone()).context("can't parse wallet after attempting to reset: refusing to overwrite existing wallet file")?;

                tracing::debug!("overwriting previous client state");

                // Overwrite the existing wallet state file, *atomically*
                std::fs::rename(&tmp_path, &wallet_path)?;

                None
            }
        };

        // If a new wallet should be saved to disk, save it and also archive it in the archive directory
        if let Some(state) = state {
            // Never overwrite a wallet that already exists
            if wallet_path.exists() {
                return Err(anyhow::anyhow!(
                    "Wallet path {} already exists, refusing to overwrite it",
                    wallet_path.display()
                ));
            }

            println!("Saving wallet to {}", wallet_path.display());
            ClientStateFile::save(state.clone(), wallet_path)?;

            // Archive the newly generated state
            let archive_dir = ProjectDirs::from("zone", "penumbra", "penumbra-testnet-archive")
                .expect("can access penumbra-testnet-archive dir");

            // Create the directory <data dir>/penumbra-testnet-archive/<chain id>/<spend key hash prefix>/
            let spend_key_hash = Sha256::digest(&state.wallet().spend_key().seed().0);
            let wallet_archive_dir = archive_dir
                .data_dir()
                .join(hex::encode(&spend_key_hash[0..8]));
            std::fs::create_dir_all(&wallet_archive_dir)
                .expect("can create penumbra wallet archive directory");

            // Save the wallet file in the archive directory
            let archive_path = wallet_archive_dir.join("penumbra_wallet.json");
            println!("Saving backup wallet to {}", archive_path.display());
            ClientStateFile::save(state, archive_path)?;
        }

        Ok(())
    }
}
