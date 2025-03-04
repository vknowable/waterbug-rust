use error::WaterbugError;
use shielded_sync::sync;
use shielded_sync::ProgressBarCallback;
use android_shielded_utils::AndroidShieldedUtils;
use namada_sdk::{
    args::TxBuilder, chain::ChainId, io::NullIo, rpc, time::DateTimeUtc, wallet::fs::FsWalletUtils, NamadaImpl,
};
use once_cell::sync::Lazy;
use std::{str::FromStr, sync::Arc};
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::{runtime::Runtime, sync::RwLock};

pub mod error;
pub mod android_shielded_utils;
pub mod shielded_sync;

uniffi::setup_scaffolding!();

// Create a global Tokio runtime
static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

// Global Namada sdk instance
static SDK_INSTANCE: Lazy<
    RwLock<Option<Arc<NamadaImpl<HttpClient, FsWalletUtils, AndroidShieldedUtils, NullIo>>>>,
> = Lazy::new(|| RwLock::new(None));

/// Initialize the global Namada sdk instance
/// rpc_url: Tendermint rpc url
/// base_dir: Android app data directory
/// cache_dir: Android app cache directory (for caching shielded sync data)
#[uniffi::export]
pub fn init_sdk(rpc_url: String, base_dir: String, cache_dir: String) -> Result<String, WaterbugError> {
    RUNTIME.block_on(async {
        // Get the chain id from the tendermint rpc
        let url = Url::from_str(rpc_url.as_str())?;
        let http_client = HttpClient::new(url.clone())?;
        let tendermint_chain = http_client.status().await?.node_info.network;
        let chain_id = ChainId::from_str(tendermint_chain.as_str())?;

        // Initialize the Namada sdk
        let wallet = FsWalletUtils::new(base_dir.as_str().into());
        let sw =
            AndroidShieldedUtils::new(base_dir.as_str().into(), cache_dir.as_str().into()).await?;
        let null_io = NullIo;
        let sdk = NamadaImpl::new(http_client, wallet, sw, null_io)
            .await?
            .chain_id(chain_id.clone());

        *SDK_INSTANCE.write().await = Some(Arc::new(sdk));
        Ok(chain_id.to_string())
    })
}

/// Query the current epoch
#[uniffi::export]
pub fn query_epoch() -> Result<u64, WaterbugError> {
    RUNTIME.block_on(async {
        let sdk = SDK_INSTANCE.read().await;
        if let Some(sdk) = &*sdk {
            let epoch = rpc::query_epoch(&sdk.clone_client()).await?;
            Ok(epoch.0)
        } else {
            Err(WaterbugError::SdkNotInitError)
        }
    })
}

#[derive(uniffi::Record)]
pub struct EpochTimeInfo {
    pub seconds_left: u64,
    pub epoch_duration: u64,
}

/// Query the seconds remaining until the next epoch. Returns a tuple of seconds remaining and the epoch duration
#[uniffi::export]
pub fn query_epoch_secs_remaining() -> Result<EpochTimeInfo, WaterbugError> {
    RUNTIME.block_on(async {
        let sdk = SDK_INSTANCE.read().await;
        if let Some(sdk) = &*sdk {
            let (this_epoch_first_height, epoch_duration) = rpc::query_next_epoch_info(&sdk.clone_client()).await?;
            let this_epoch_first_height_header = rpc::query_block_header(&sdk.clone_client(), this_epoch_first_height).await?.unwrap();
            let first_block_time = this_epoch_first_height_header.time;
            let next_epoch_time = first_block_time + epoch_duration.min_duration;
            let current_time = DateTimeUtc::now();
            let seconds_left = next_epoch_time.time_diff(current_time).0;
            Ok(EpochTimeInfo {
                seconds_left,
                epoch_duration: epoch_duration.min_duration.0,
            })
        } else {
            Err(WaterbugError::SdkNotInitError)
        }
    })
}

#[uniffi::export]
pub fn shielded_sync(masp_indexer_url: String, viewing_key: String, callback: Arc<dyn ProgressBarCallback>) -> String {
    RUNTIME.block_on(async {
        let sdk = SDK_INSTANCE.read().await;
        if let Some(sdk) = &*sdk {
            match sync::<HttpClient, FsWalletUtils, AndroidShieldedUtils, NullIo>(
                sdk,
                masp_indexer_url,
                viewing_key,
                callback,
            )
            .await
            {
                Ok(_) => return "sync successful".to_string(),
                Err(e) => return format!("sync error: {e}"),
            }
        } else {
            "SDK not initialized".to_string()
        }
    })
}