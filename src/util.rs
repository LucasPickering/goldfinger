use anyhow::{Context, anyhow};
use log::{error, info, warn};
use serde::de::DeserializeOwned;
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

/// Helper for regularly fetching data from an API
#[derive(Debug)]
pub struct ApiFetcher<T> {
    url: String,
    /// Frequency at which to refetch data
    ttl: Duration,
    /// Data loaded from the API. The load is done in a separate thread and
    /// deposited here
    data: Arc<RwLock<Option<(T, Instant)>>>,
}

impl<T> ApiFetcher<T>
where
    T: 'static + Clone + DeserializeOwned + Send + Sync,
{
    pub fn new(url: String, ttl: Duration) -> Self {
        Self {
            url,
            ttl,
            data: Default::default(),
        }
    }

    /// Get the latest data. If the data is missing or outdated, spawn a task to
    /// re-fetch it
    pub fn data(&self) -> Option<T> {
        let Some(guard) = self.data.try_read().ok() else {
            // Content is so low that we don't ever expect to hit this
            warn!("Failed to grab data read lock");
            return None;
        };

        if let Some((data, fetched_at)) = guard.as_ref() {
            // If forecast is stale, fetch a new one in the background
            if *fetched_at + self.ttl < Instant::now() {
                self.fetch_latest();
            }

            // Return the data even if it's old. Old is better than nothing.
            // Clone so we can release the lock
            Some(data.clone())
        } else {
            self.fetch_latest();
            None
        }
    }

    /// Spawn a task to fetch the latest forecase in the background
    fn fetch_latest(&self) {
        let lock = Arc::clone(&self.data);
        let url = self.url.clone();
        let request = ureq::get(&url);

        thread::spawn(move || {
            // Shitty try block
            let result: anyhow::Result<()> = (|| {
                info!("Fetching new data from {url}");
                let mut response = request.call().with_context(|| {
                    format!("Error fetching data from {url}")
                })?;
                let data: T =
                    response.body_mut().read_json().with_context(|| {
                        format!("Error parsing data from {url} as JSON")
                    })?;

                // Stringify the error to dump the lifetime
                *lock.write().map_err(|err| anyhow!("{err}"))? =
                    Some((data, Instant::now()));
                Ok(())
            })();

            if let Err(err) = result {
                error!("Error fetching data from {url}: {err:?}")
            }
        });
    }
}
