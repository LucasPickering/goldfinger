use anyhow::{anyhow, Context};
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

const FORECAST_URL: &str =
    "https://api.weather.gov/gridpoints/BOX/71,90/forecast";

/// Gotta know weather or not it's gonna rain
#[derive(Debug, Default)]
pub struct Weather {
    forecast: Arc<RwLock<Option<(Forecast, Instant)>>>,
}

impl Weather {
    const FORECAST_TTL: Duration = Duration::from_secs(600);

    /// Get the latest forecast. If the forecast is missing or outdated, spawn
    /// a task to re-fetch it
    pub fn forecast(&self) -> Option<Forecast> {
        let Some(guard) = self.forecast.try_read().ok() else {
            // Content is so low that we don't ever expect to hit this
            warn!("Failed to grab forecast read lock");
            return None;
        };

        if let Some((forecast, fetched_at)) = guard.as_ref() {
            // If forecast is stale, fetch a new one in the background
            if *fetched_at + Self::FORECAST_TTL < Instant::now() {
                self.fetch_latest();
            }

            // Return the forecast even if it's old. Old is better than nothing
            // Clone the forecast so we can release the lock
            Some(forecast.clone())
        } else {
            self.fetch_latest();
            None
        }
    }

    /// Spawn a task to fetch the latest forecase in the background
    fn fetch_latest(&self) {
        let lock = Arc::clone(&self.forecast);
        thread::spawn(move || {
            // Shitty try block
            let result: anyhow::Result<()> = (|| {
                info!("Fetching new forecast");
                let response =
                    ureq::get(FORECAST_URL).call().with_context(|| {
                        format!("Error fetching forecast from {FORECAST_URL}")
                    })?;
                let forecast: Forecast = response
                    .into_json()
                    .context("Error parsing forecast as JSON")?;
                info!("Saving forecast");
                let now = Instant::now();
                // Stringify the error to dump the lifetime
                *lock.write().map_err(|err| anyhow!("{err}"))? =
                    Some((forecast, now));
                Ok(())
            })();

            if let Err(err) = result {
                error!("Error fetching forecast: {err:?}")
            }
        });
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub properties: ForecastProperties,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastProperties {
    pub periods: Vec<ForecastPeriod>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPeriod {
    pub name: String,
    pub temperature: i32,
    pub short_forecast: String,
    pub probability_of_precipitation: Unit,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub value: Option<i32>,
}

impl ForecastPeriod {
    /// Percentage chance of precipitation
    pub fn precipitation(&self) -> i32 {
        self.probability_of_precipitation.value.unwrap_or_default()
    }
}
