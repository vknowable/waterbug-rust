use error::WaterbugError;
use namada_sdk::{
    args::TxBuilder, chain::ChainId, io::NullIo, masp::{fs::FsShieldedUtils, ContextSyncStatus}, rpc, wallet::fs::FsWalletUtils, NamadaImpl, ShieldedWallet
};
use once_cell::sync::Lazy;
use std::{str::FromStr, sync::Arc};
use tendermint_rpc::{Client, HttpClient, Url};
use tokio::{runtime::Runtime, sync::RwLock};

pub mod error;

uniffi::setup_scaffolding!();

// Create a global Tokio runtime
static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

// Global Namada sdk instance
static SDK_INSTANCE: Lazy<
    RwLock<Option<Arc<NamadaImpl<HttpClient, FsWalletUtils, FsShieldedUtils, NullIo>>>>,
> = Lazy::new(|| RwLock::new(None));

/// Initialize the global Namada sdk instance
/// rpc_url: Tendermint rpc url
/// base_dir: Android app data directory
/// cache_dir: Android app cache directory (for caching shielded sync data)
#[uniffi::export]
pub fn init_sdk(rpc_url: String, base_dir: String, cache_dir: String) -> Result<(), WaterbugError> {
    RUNTIME.block_on(async {
        // Get the chain id from the tendermint rpc
        let url = Url::from_str(rpc_url.as_str())?;
        let http_client = HttpClient::new(url.clone())?;
        let tendermint_chain = http_client.status().await?.node_info.network;
        let chain_id = ChainId::from_str(tendermint_chain.as_str())?;

        // Initialize the Namada sdk
        let wallet = FsWalletUtils::new(base_dir.as_str().into());
        let shielded_ctx = FsShieldedUtils::default();
        let sw = ShieldedWallet {
            utils: shielded_ctx,
            sync_status: ContextSyncStatus::Speculative,
            ..Default::default()
        };
        // let sw = FsShieldedUtils::new(base_dir.as_str().into());
        // let sw =
        //     AndroidShieldedUtils::new(base_dir.as_str().into(), cache_dir.as_str().into()).await;
        let null_io = NullIo;
        let sdk = NamadaImpl::new(http_client, wallet, sw, null_io)
            .await?
            .chain_id(chain_id.clone());

        *SDK_INSTANCE.write().await = Some(Arc::new(sdk));
        Ok(())
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

// #[uniffi::export]
// pub fn add(left: u64, right: u64, error: bool) -> Result<u64, WaterbugError> {
//     if error {
//         Err(WaterbugError::TestError("Test error".to_string()))
//     } else {
//         Ok(left + right)
//     }
// }
