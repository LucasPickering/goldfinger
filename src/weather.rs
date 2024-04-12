use crate::config::Config;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Local, NaiveTime, Utc};
use log::{error, info, warn};
use serde::Deserialize;
use std::{
    fs,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

/// Gotta know weather or not it's gonna rain
#[derive(Debug)]
pub struct Weather {
    api_key: String,
    /// Latitude, longitude
    location: (f32, f32),
    forecast: Arc<RwLock<Option<(Forecast, Instant)>>>,
}

impl Weather {
    const FORECAST_TTL: Duration = Duration::from_secs(600);
    const API_URL: &'static str =
        "https://api.openweathermap.org/data/2.5/forecast";

    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let api_key = fs::read_to_string(&config.openweather_token_file)
            .context(format!(
                "Error reading OpenWeather token from {:?}",
                config.openweather_token_file,
            ))?
            .trim()
            .to_owned();
        Ok(Self {
            api_key,
            location: config.weather_location,
            forecast: Default::default(),
        })
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
        let (lat, lon) = self.location;
        let request = ureq::get(Self::API_URL).query_pairs([
            ("lat", lat.to_string().as_str()),
            ("lon", lon.to_string().as_str()),
            ("units", "imperial"), // 🇺🇸
            // Yes this is really how they auth...
            ("appid", self.api_key.as_str()),
        ]);

        thread::spawn(move || {
            // Shitty try block
            let result: anyhow::Result<()> = (|| {
                info!("Fetching new forecast");
                let response = request.call().with_context(|| {
                    format!("Error fetching forecast from {}", Self::API_URL)
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

/// https://openweathermap.org/forecast5#fields_JSON
#[derive(Clone, Debug, Deserialize)]
pub struct Forecast {
    pub list: Vec<ForecastPeriod>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ForecastPeriod {
    /// Private to force usage of the localized version
    #[serde(rename = "dt", with = "chrono::serde::ts_seconds")]
    time: DateTime<Utc>,
    pub main: ForecastPeriodMain,
    #[serde(rename = "pop")]
    pub prob_of_precip: f32,
    pub weather: Vec<WeatherItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ForecastPeriodMain {
    #[serde(rename = "temp")]
    pub temperature: f32,
    pub feels_like: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeatherItem {
    pub main: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Wind {
    pub deg: u32,
    pub gust: f32,
    pub speed: f32,
}

impl Forecast {
    /// Get the list of periods that should be shown. This skips ones in the
    /// middle of the night
    pub fn periods(&self) -> impl Iterator<Item = &ForecastPeriod> {
        let night_start = NaiveTime::from_hms_opt(22, 0, 0).unwrap();
        let night_end = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
        self.list.iter().filter(move |period| {
            // Time is a flat circle, so we can't use Range::contains
            let time = period.time().time();
            night_end <= time && time < night_start
        })
    }
}

impl ForecastPeriod {
    /// Localized timestamp for this period
    pub fn time(&self) -> DateTime<Local> {
        self.time.with_timezone(&Local)
    }

    /// Formatted temperature
    pub fn temperature(&self) -> String {
        format!("{:.0}°", self.main.temperature)
    }

    /// Formatted probability of precipitation
    pub fn prob_of_precip(&self) -> String {
        format!("{:.0}%", self.prob_of_precip)
    }

    /// Get the name of the weather for this period, e.g. "clear" or "clouds"
    pub fn weather(&self) -> &str {
        &self.weather[0].main
    }
}
