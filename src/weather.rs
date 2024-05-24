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
    /// Number of weather periods to join together in [Forecase::future_periods]
    const JOINED_PERIODS: usize = 3;

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

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPeriod {
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    temperature: i32,
    probability_of_precipitation: Unit,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub value: Option<i32>,
}

impl Forecast {
    /// Get the current forecast period
    pub fn now(&self) -> &ForecastPeriod {
        &self.properties.periods[0]
    }

    /// Get the list of *future* periods that should be shown. This skips
    /// periods in the middle of the night, as well as the current period.
    /// Windows will be joined together, with [Weather::JOINED_PERIODS].
    pub fn future_periods(&self) -> impl '_ + Iterator<Item = ForecastPeriod> {
        let day_range = Weather::DAY_START.unwrap()..=Weather::DAY_END.unwrap();
        // Cut out the current period
        let periods = &self.properties.periods[1..];
        periods
            .chunks(Weather::JOINED_PERIODS)
            .map(ForecastPeriod::join)
            .filter(move |period| {
                // Include if *any* part of the period is in daytime
                day_range.contains(&period.start_time().time())
                    || day_range.contains(&period.end_time().time())
            })
    }
}

impl ForecastPeriod {
    /// Join the given periods by averaging their values
    pub fn join(periods: &[Self]) -> Self {
        let average = |f: fn(&Self) -> i32| {
            periods.iter().map(f).sum::<i32>() / periods.len() as i32
        };

        ForecastPeriod {
            // Assume slice is non-empty and sorted chronologically
            start_time: periods.first().unwrap().start_time,
            end_time: periods.last().unwrap().end_time,

            temperature: average(|period| period.temperature),
            probability_of_precipitation: Unit {
                value: Some(average(|period| {
                    period
                        .probability_of_precipitation
                        .value
                        .unwrap_or_default()
                })),
            },
        }
    }

    /// Localized timestamp for the start of this period
    pub fn start_time(&self) -> DateTime<Local> {
        self.start_time.with_timezone(&Local)
    }

    /// Localized timestamp for the end of this period
    pub fn end_time(&self) -> DateTime<Local> {
        self.end_time.with_timezone(&Local)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn period(
        time: &str,
        hours: i64,
        temperature: i32,
        probability_of_precipitation: i32,
    ) -> ForecastPeriod {
        let start_time = time.parse().unwrap();
        let end_time = start_time + chrono::Duration::hours(hours);
        ForecastPeriod {
            start_time,
            end_time,
            temperature,
            probability_of_precipitation: Unit {
                value: Some(probability_of_precipitation),
            },
        }
    }

    // TODO fix tz thing

    #[test]
    fn test_now() {
        let forecast = Forecast {
            properties: ForecastProperties {
                periods: vec![
                    period("2024-05-24T17:00:00Z", 1, 84, 1),
                    period("2024-05-24T18:00:00Z", 1, 85, 0),
                    period("2024-05-24T19:00:00Z", 1, 86, 0),
                ],
            },
        };

        assert_eq!(forecast.now(), &period("2024-05-24T17:00:00Z", 1, 84, 1));
    }

    #[test]
    fn test_future_periods() {
        // Make timezone conversions consistent
        std::env::set_var("TZ", "UTC");

        let forecast = Forecast {
            properties: ForecastProperties {
                periods: vec![
                    period("2024-05-24T17:00:00Z", 1, 84, 1), /* First is skipped */
                    //
                    period("2024-05-24T18:00:00Z", 1, 85, 0),
                    period("2024-05-24T19:00:00Z", 1, 86, 0),
                    period("2024-05-24T20:00:00Z", 1, 87, 0),
                    //
                    period("2024-05-24T21:00:00Z", 1, 85, 10),
                    period("2024-05-24T22:00:00Z", 1, 84, 20),
                    period("2024-05-24T23:00:00Z", 1, 82, 30),
                    // vvv Excluded because entirely nightttime vvv
                    period("2024-05-25T00:00:00Z", 1, 78, 0),
                    period("2024-05-25T01:00:00Z", 1, 75, 0),
                    period("2024-05-25T02:00:00Z", 1, 72, 0),
                    // ^^^ Excluded ^^^
                    period("2024-05-25T03:00:00Z", 1, 69, 0),
                    period("2024-05-25T04:00:00Z", 1, 67, 0),
                    period("2024-05-25T05:00:00Z", 1, 65, 30),
                    //
                    period("2024-05-25T06:00:00Z", 1, 63, 0),
                    period("2024-05-25T07:00:00Z", 1, 60, 0),
                    period("2024-05-25T08:00:00Z", 1, 58, 0),
                    //
                    period("2024-05-25T09:00:00Z", 1, 57, 0),
                ],
            },
        };

        let periods: Vec<_> = forecast.future_periods().collect();
        assert_eq!(
            periods.as_slice(),
            &[
                period("2024-05-24T18:00:00Z", 3, 86, 0),
                period("2024-05-24T21:00:00Z", 3, 83, 20),
                period("2024-05-25T03:00:00Z", 3, 67, 10),
                period("2024-05-25T06:00:00Z", 3, 60, 0),
                period("2024-05-25T09:00:00Z", 1, 57, 0),
            ]
        );
    }
}
