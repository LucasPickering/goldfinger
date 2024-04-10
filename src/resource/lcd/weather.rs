use anyhow::Context;
use log::{error, info, warn};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::RwLock, task};

const FORECAST_URL: &str =
    "https://api.weather.gov/gridpoints/BOX/71,90/forecast";

/// Gotta know weather or not it's gonna rain
#[derive(Debug)]
pub struct Weather {
    client: Client,
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
        let client = self.client.clone();
        let lock = Arc::clone(&self.forecast);
        task::spawn(async move {
            // Shitty try block
            let result: anyhow::Result<Forecast> = async move {
                info!("Fetching new forecast");
                let response = client
                    .get(FORECAST_URL)
                    .send()
                    .await
                    .with_context(|| {
                        format!("Error fetching forecast from {FORECAST_URL}")
                    })?;
                response
                    .json()
                    .await
                    .context("Error parsing forecast as JSON")
            }
            .await;

            match result {
                Ok(forecast) => {
                    info!("Saving forecast");
                    let now = Instant::now();
                    *lock.write().await = Some((forecast, now));
                }
                Err(err) => {
                    error!("Error fetching forecast: {err:?}")
                }
            }
        });
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            client: ClientBuilder::new()
                .user_agent("goldfinger")
                .build()
                .unwrap(),
            forecast: Default::default(),
        }
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

impl Display for ForecastPeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{:} {}\u{272} {}%",
            self.name,
            self.temperature,
            self.probability_of_precipitation.value.unwrap_or_default(),
        )?;
        write!(f, "{}", self.short_forecast)?;
        Ok(())
    }
}
