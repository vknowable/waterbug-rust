use std::{sync::Arc, time::Duration, str::FromStr};
use namada_sdk::{
    control_flow::ShutdownSignal, error::Error, io::{NullIo, ProgressBar}, masp::{utils::RetryStrategy, IndexerMaspClient, MaspLocalTaskEnv, ShieldedSyncConfig}, masp_primitives::zip32::ExtendedFullViewingKey, state::BlockHeight, wallet::{fs::FsWalletUtils, DatedKeypair}, ExtendedViewingKey, MaybeSend, MaybeSync, Namada, NamadaImpl 
    
};
use tendermint_rpc::HttpClient;

use crate::android_shielded_utils::AndroidShieldedUtils;

#[uniffi::export(with_foreign)]
pub trait ProgressBarCallback: MaybeSend + MaybeSync {
    fn message(&self, name: String, msg: String);
    fn on_progress_started(&self, name: String, total: i32);
    fn on_progress_incremented(&self, name: String, current: i32, total: i32);
    fn on_progress_complete(&self, name: String);
}

#[derive(Clone)]
pub struct ProgressBarAndroid {
    pub name: String,
    pub total: usize,
    pub current: usize,
    callback: Arc<dyn ProgressBarCallback>,
}

impl ProgressBarAndroid {
    pub fn new(name: String, callback: Arc<dyn ProgressBarCallback>) -> Self {
        Self {
            name,
            total: 0,
            current: 0,
            callback,
        }
    }
}

impl ProgressBar for ProgressBarAndroid {
    fn upper_limit(&self) -> u64 {
        self.total as u64
    }

    fn set_upper_limit(&mut self, limit: u64) {
        self.total = limit as usize;
        self.callback.on_progress_started(self.name.clone(), self.total as i32);
    }

    fn increment_by(&mut self, amount: u64) {
        self.current += amount as usize;
        self.callback.on_progress_incremented(self.name.clone(), self.current as i32, self.total as i32);
    }

    fn message(&mut self, message: String) {
        self.callback.message(self.name.clone(), message);
    }

    fn finish(&mut self) {
        self.callback.on_progress_complete(self.name.clone());
    }
}

// Dummy shutdown signal
pub struct ShutdownSignalAndroid {}

impl ShutdownSignal for ShutdownSignalAndroid {
    async fn wait_for_shutdown(&mut self) {}

    fn received(&mut self) -> bool {
        false
    }
}

pub async fn sync<C, U, V, I>(
    sdk: &Arc<NamadaImpl<HttpClient, FsWalletUtils, AndroidShieldedUtils, NullIo>>,
    masp_indexer_url: String,
    viewing_key: String,
    callback: Arc<dyn ProgressBarCallback>,
) -> Result<(), Error> {
    let progress_bar_scanned = ProgressBarAndroid::new("ProgressBarScanned".to_string(), callback.clone());
    let progress_bar_fetched = ProgressBarAndroid::new("ProgressBarFetched".to_string(), callback.clone());
    let progress_bar_applied = ProgressBarAndroid::new("ProgressBarApplied".to_string(), callback.clone());
    let shutdown_signal_android = ShutdownSignalAndroid {};

    // create a thread pool for the shielded sync
    let env = MaspLocalTaskEnv::new(10)
        .map_err(|err| Error::Other(format!("Error creating task environment: {err}")))?;

    // create a masp client to sync from the masp-indexer
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(60))
        .build()
        .map_err(|err| Error::Other(format!("Failed to build http client: {err}")))?;

    let url = masp_indexer_url.as_str().try_into().map_err(|err| {
        Error::Other(format!(
            "Failed to parse API endpoint {masp_indexer_url:?}: {err}"
        ))
    })?;

    let shielded_client = IndexerMaspClient::new(client, url, true, 100);

    let config = ShieldedSyncConfig::builder()
        .client(shielded_client)
        .scanned_tracker(progress_bar_scanned)
        .fetched_tracker(progress_bar_fetched)
        .applied_tracker(progress_bar_applied)
        .shutdown_signal(shutdown_signal_android)
        .block_batch_size(100)
        .wait_for_last_query_height(true)
        .retry_strategy(RetryStrategy::Times(10))
        .build();

    let dated_keypair = DatedKeypair {
        key: ExtendedFullViewingKey::from(
            ExtendedViewingKey::from_str(viewing_key.as_str()).expect("Could not parse viewing key"),
        )
        .fvk
        .vk,
        birthday: BlockHeight::from(0),
    };

    sdk.shielded_mut()
        .await
        .sync(env, config, None, &[], &[dated_keypair])
        .await
        .map_err(|err| Error::Other(format!("Shielded-sync error: {err}")))?;
    Ok(())
}