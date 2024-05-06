use crate::config::Config;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Local, NaiveTime, Utc};
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

/// Gotta know weather or not it's gonna rain
#[derive(Debug)]
pub struct Weather {
    url: String,
    forecast: Arc<RwLock<Option<(Forecast, Instant)>>>,
}

impl Weather {
    const FORECAST_TTL: Duration = Duration::from_secs(600);
    const API_HOST: &'static str = "https://api.weather.gov";
    // Start and end (inclusive) of forecast times that *should* be shown.
    // unstable: const unwrap https://github.com/rust-lang/rust/issues/67441
    const DAY_START: Option<NaiveTime> = NaiveTime::from_hms_opt(4, 30, 0);
    const DAY_END: Option<NaiveTime> = NaiveTime::from_hms_opt(22, 30, 0);

    pub fn new(config: &Config) -> Self {
        let url = format!(
            "{}/gridpoints/{}/{},{}/forecast/hourly",
            Self::API_HOST,
            config.forecast_office,
            config.forecast_gridpoint.0,
            config.forecast_gridpoint.1
        );
        Self {
            url,
            forecast: Default::default(),
        }
    }

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
        let request = ureq::get(&self.url);

        thread::spawn(move || {
            // Shitty try block
            let result: anyhow::Result<()> = (|| {
                info!("Fetching new forecast");
                let response = request.call().with_context(|| {
                    format!("Error fetching forecast from {}", Self::API_HOST)
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

///https://www.weather.gov/documentation/services-web-api#/default/gridpoint_forecast
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    properties: ForecastProperties,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastProperties {
    periods: Vec<ForecastPeriod>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPeriod {
    start_time: DateTime<Utc>,
    temperature: i32,
    probability_of_precipitation: Unit,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub value: Option<i32>,
}

impl Forecast {
    /// Get the current forecast period
    pub fn now(&self) -> &ForecastPeriod {
        &self.properties.periods[0]
    }

    /// Get the list of *future* periods that should be shown. This skips ones
    /// in the middle of the night, as well as the current period
    pub fn future_periods(&self) -> impl Iterator<Item = &ForecastPeriod> {
        let day_range = Weather::DAY_START.unwrap()..=Weather::DAY_END.unwrap();
        self.properties
            .periods
            .iter()
            .skip(1) // FUUUUUUUUTUUUUUURE
            .filter(move |period| {
                day_range.contains(&period.start_time().time())
            })
    }
}

impl ForecastPeriod {
    /// Localized timestamp for the start of this period
    pub fn start_time(&self) -> DateTime<Local> {
        self.start_time.with_timezone(&Local)
    }

    /// Formatted temperature
    pub fn temperature(&self) -> String {
        format!("{:.0}°", self.temperature)
    }

    /// Formatted probability of precipitation
    pub fn prob_of_precip(&self) -> String {
        format!(
            "{:.0}%",
            self.probability_of_precipitation.value.unwrap_or_default()
        )
    }
}
