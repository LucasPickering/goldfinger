use crate::resource::lcd::{LcdText, LCD_WIDTH};
use anyhow::Context;
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::{
    cmp,
    fmt::{self, Display, Formatter},
};

const FORECAST_URL: &str =
    "https://api.weather.gov/gridpoints/BOX/71,90/forecast";

/// Gotta know weather or not it's gonna rain
#[derive(Debug)]
pub struct Weather {
    client: Client,
}

impl Weather {
    pub async fn forecast(&self) -> anyhow::Result<LcdText> {
        let response = self
            .client
            .get(FORECAST_URL)
            .send()
            .await
            .with_context(|| {
                format!("Error fetching forecast from {FORECAST_URL}")
            })?;
        let forecast: Forecast = response.json().await?;
        let periods = forecast.properties.periods;
        // Show the first two forecast periods
        let text = format!("{}\n{}", periods[0], periods[1]);
        Ok(text.as_str().into())
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            client: ClientBuilder::new()
                .user_agent("goldfinger")
                .build()
                .unwrap(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Forecast {
    properties: ForecastProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastProperties {
    periods: Vec<ForecastPeriod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastPeriod {
    name: String,
    temperature: i32,
    short_forecast: String,
    probability_of_precipitation: Unit,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Unit {
    value: Option<i32>,
}

impl Display for ForecastPeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Shorten period name to max sure temp/precip fits
        let mut temp_precip = format!(
            "{}\u{00} {}%",
            self.temperature,
            self.probability_of_precipitation.value.unwrap_or_default(),
        );

        // Hack to insert the degree symbol. The LCD uses bytes 128-255 for
        // non-ASCII characters, which is not valid UTF-8 (UTF-8 uses 2 bytes
        // for anything over 127). Null char is hard-coded to be a degree symbol
        unsafe {
            for byte in temp_precip.as_bytes_mut() {
                if *byte == 0 {
                    *byte = 0xdf;
                }
            }
        }

        // Shorten name if necessary to fit in available space
        let name_max_length = LCD_WIDTH - temp_precip.len() - 1;
        let short_name =
            &self.name[..cmp::min(self.name.len(), name_max_length)];

        writeln!(
            f,
            "{:<name_max_length$} {}",
            short_name,
            temp_precip,
            name_max_length = name_max_length
        )?;
        write!(f, " {}", self.short_forecast)
    }
}
