// This is basically a straight copy/paste of FsShieldedUtils from the namada sdk, with some minor changes to handling the default paths and 
// masp-params downloading (which is now handled natively in the android app)

use borsh::{BorshDeserialize, BorshSerialize};
use std::env;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use namada_sdk::{
    masp::{ContextSyncStatus, DispatcherCache},
    masp_proofs::prover::LocalTxProver,
    ShieldedWallet,
    ShieldedUtils,
    MaybeSend, MaybeSync,
};

// Shielded context file names
const FILE_NAME: &str = "shielded.dat";
const TMP_FILE_PREFIX: &str = "shielded.tmp";
const SPECULATIVE_FILE_NAME: &str = "speculative_shielded.dat";
const SPECULATIVE_TMP_FILE_PREFIX: &str = "speculative_shielded.tmp";
const CACHE_FILE_NAME: &str = "shielded_sync.cache";
const CACHE_FILE_TMP_PREFIX: &str = "shielded_sync.cache.tmp";
use namada_sdk::masp::{CONVERT_NAME, OUTPUT_NAME, SPEND_NAME};

use crate::error::WaterbugError;

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
/// An implementation of ShieldedUtils for standard filesystems
pub struct AndroidShieldedUtils {
    #[borsh(skip)]
    pub(crate) context_dir: PathBuf,
    #[borsh(skip)]
    pub(crate) cache_dir: PathBuf,
}

impl AndroidShieldedUtils {
    /// Initialize a shielded transaction context that identifies notes
    /// decryptable by any viewing key in the given set
    pub async fn new(context_dir: PathBuf, cache_dir: PathBuf) -> Result<ShieldedWallet<Self>, WaterbugError> {
        // Make sure that MASP parameters are downloaded to enable MASP
        // transaction building and verification later on
        // (by this point, they should have already been downloaded natively in the android app)
        let params_dir = context_dir.clone();
        let spend_path = params_dir.join(SPEND_NAME);
        let convert_path = params_dir.join(CONVERT_NAME);
        let output_path = params_dir.join(OUTPUT_NAME);
        
        // TODO: uncomment this check when finished debugging
        // if !(spend_path.exists() && convert_path.exists() && output_path.exists()) {
        //     return Err(WaterbugError::NamadaSdkError("MASP parameters not present or downloadable".to_string()));
        // }

        // Finally initialize a shielded context with the supplied directory
        let sync_status = if std::fs::read(context_dir.join(SPECULATIVE_FILE_NAME)).is_ok() {
            // Load speculative state
            ContextSyncStatus::Speculative
        } else {
            ContextSyncStatus::Confirmed
        };

        let utils = Self {
            context_dir,
            cache_dir,
        };
        Ok(ShieldedWallet {
            utils,
            sync_status,
            ..Default::default()
        })
    }

    /// Write to a file ensuring that all contents of the file
    /// were written by a single process (in case of multiple
    /// concurrent write attempts).
    ///
    /// N.B. This is not the same as a file lock. If multiple
    /// concurrent writes take place, this code ensures that
    /// the result of exactly one will be persisted.
    ///
    /// N.B. This only truly works if each process uses
    /// to a *unique* tmp file name.
    fn atomic_file_write(
        &self,
        tmp_file_name: impl AsRef<std::path::Path>,
        file_name: impl AsRef<std::path::Path>,
        data: impl BorshSerialize,
    ) -> std::io::Result<()> {
        let tmp_path = self.context_dir.join(&tmp_file_name);
        {
            // First serialize the shielded context into a temporary file.
            // Inability to create this file implies a simultaneuous write
            // is in progress. In this case, immediately
            // fail. This is unproblematic because the data
            // intended to be stored can always be re-fetched
            // from the blockchain.
            let mut ctx_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(tmp_path.clone())?;
            let mut bytes = Vec::new();
            data.serialize(&mut bytes).unwrap_or_else(|e| {
                panic!(
                    "cannot serialize data to {} with error: {}",
                    file_name.as_ref().to_string_lossy(),
                    e,
                )
            });
            ctx_file.write_all(&bytes[..])?;
        }
        // Atomically update the old shielded context file with new data.
        // Atomicity is required to prevent other client instances from
        // reading corrupt data.
        std::fs::rename(tmp_path, self.context_dir.join(file_name))
    }
}

impl Default for AndroidShieldedUtils {
    fn default() -> Self {
        Self {
            context_dir: PathBuf::from(FILE_NAME),
            cache_dir: PathBuf::from(FILE_NAME),
        }
    }
}

// #[cfg_attr(feature = "async-send", async_trait::async_trait)]
// #[cfg_attr(not(feature = "async-send"), async_trait::async_trait(?Send))]
#[async_trait::async_trait(?Send)]
impl ShieldedUtils for AndroidShieldedUtils {
    fn local_tx_prover(&self) -> LocalTxProver {
        let params_dir = self.context_dir.clone();
        let spend_path = params_dir.join(SPEND_NAME);
        let convert_path = params_dir.join(CONVERT_NAME);
        let output_path = params_dir.join(OUTPUT_NAME);
        LocalTxProver::new(&spend_path, &output_path, &convert_path)
    }

    /// Try to load the last saved shielded context from the given context
    /// directory. If this fails, then leave the current context unchanged.
    async fn load<U: ShieldedUtils + MaybeSend>(
        &self,
        ctx: &mut ShieldedWallet<U>,
        force_confirmed: bool,
    ) -> std::io::Result<()> {
        // Try to load shielded context from file
        let file_name = if force_confirmed {
            FILE_NAME
        } else {
            match ctx.sync_status {
                ContextSyncStatus::Confirmed => FILE_NAME,
                ContextSyncStatus::Speculative => SPECULATIVE_FILE_NAME,
            }
        };
        let mut ctx_file = File::open(self.context_dir.join(file_name))?;
        let mut bytes = Vec::new();
        ctx_file.read_to_end(&mut bytes)?;
        // Fill the supplied context with the deserialized object
        *ctx = ShieldedWallet {
            utils: ctx.utils.clone(),
            ..ShieldedWallet::<U>::deserialize(&mut &bytes[..])?
        };
        Ok(())
    }

    /// Save this confirmed shielded context into its associated context
    /// directory. At the same time, delete the speculative file if present
    async fn save<U: ShieldedUtils + MaybeSync>(
        &self,
        ctx: &ShieldedWallet<U>,
    ) -> std::io::Result<()> {
        env::set_var("TMPDIR", self.cache_dir.clone());
        let (tmp_file_pref, file_name) = match ctx.sync_status {
            ContextSyncStatus::Confirmed => (TMP_FILE_PREFIX, FILE_NAME),
            ContextSyncStatus::Speculative => (SPECULATIVE_TMP_FILE_PREFIX, SPECULATIVE_FILE_NAME),
        };
        let tmp_file_name = {
            let t = tempfile::Builder::new().prefix(tmp_file_pref).tempfile()?;
            t.path().file_name().unwrap().to_owned()
        };
        self.atomic_file_write(tmp_file_name, file_name, ctx)?;

        // Remove the speculative file if present since it's state is
        // overruled by the confirmed one we just saved
        if let ContextSyncStatus::Confirmed = ctx.sync_status {
            let _ = std::fs::remove_file(self.context_dir.join(SPECULATIVE_FILE_NAME));
        }

        Ok(())
    }

    async fn cache_save(&self, cache: &DispatcherCache) -> std::io::Result<()> {
        let tmp_file_name = {
            let t = tempfile::Builder::new()
                .prefix(CACHE_FILE_TMP_PREFIX)
                .tempfile()?;
            t.path().file_name().unwrap().to_owned()
        };

        self.atomic_file_write(tmp_file_name, CACHE_FILE_NAME, cache)
    }

    async fn cache_load(&self) -> std::io::Result<DispatcherCache> {
        let file_name = self.context_dir.join(CACHE_FILE_NAME);
        let mut file = File::open(file_name)?;
        DispatcherCache::try_from_reader(&mut file)
    }
}
