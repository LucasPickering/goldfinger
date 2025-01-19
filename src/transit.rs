use crate::{config::Config, util::ApiFetcher};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use itertools::Itertools;
use log::error;
use serde::Deserialize;
use std::{fmt::Display, time::Duration};

#[derive(Debug)]
pub struct Transit {
    fetcher: ApiFetcher<ApiPredictions>,
    lines: Vec<TransitLine>,
}

impl Transit {
    /// Time between fetches
    const DATA_TTL: Duration = Duration::from_secs(30);
    /// Max number of pending departures to show for a stop
    const MAX_PREDICTIONS: usize = 2;

    pub fn new(config: &Config) -> Self {
        let all_stops = config
            .transit_lines
            .iter()
            .flat_map(|line| [line.inbound_stop, line.outbound_stop]);
        let url = format!(
            "https://api-v3.mbta.com/predictions?filter[stop]={}",
            all_stops.format(",")
        );
        Self {
            fetcher: ApiFetcher::new(url, Self::DATA_TTL),
            lines: config.transit_lines.clone(),
        }
    }

    pub fn predictions(&self) -> Predictions {
        struct Helper {
            inbound_stop: u32,
            outbound_stop: u32,
            inbound: Vec<DateTime<Utc>>,
            outbound: Vec<DateTime<Utc>>,
        }

        // Group predictions as (line, (inbound, outbound))
        let mut grouped: IndexMap<String, Helper> = self
            .lines
            .iter()
            .map(|line| {
                (
                    line.name.clone(),
                    Helper {
                        inbound_stop: line.inbound_stop,
                        outbound_stop: line.outbound_stop,
                        inbound: Vec::new(),
                        outbound: Vec::new(),
                    },
                )
            })
            .collect();

        // Pull data from the most recent response
        if let Some(data) = self.fetcher.data() {
            for prediction in data.data {
                // Departure time will be empty if the stop is being skipped
                let Some(departure_time) = prediction.attributes.departure_time
                else {
                    continue;
                };
                let route_id = prediction.relationships.route.data.id;
                let Some(group) = grouped.get_mut(&route_id) else {
                    error!("Unknown route {route_id}");
                    continue;
                };
                let stop_id = &prediction.relationships.stop.data.id;

                if stop_id == &group.inbound_stop.to_string() {
                    group.inbound.push(departure_time);
                } else if stop_id == &group.outbound_stop.to_string() {
                    group.outbound.push(departure_time);
                } else {
                    error!("Unknown stop {stop_id} for route {route_id}");
                }
            }
        }

        // We want to show empty data if we don't have an API response yet
        let lines = grouped
            .into_iter()
            .map(|(name, data)| LinePrediction {
                name,
                inbound: data.inbound.into(),
                outbound: data.outbound.into(),
            })
            .collect();
        Predictions { lines }
    }
}

/// Configuration for a transit line to show predictions for
#[derive(Clone, Debug, Deserialize)]
pub struct TransitLine {
    pub name: String,
    /// ID of the inbound stop you care about
    pub inbound_stop: u32,
    /// ID of the outbound stop you care about
    pub outbound_stop: u32,
}

#[derive(Debug)]
pub struct Predictions {
    pub lines: Vec<LinePrediction>,
}

#[derive(Debug)]
pub struct LinePrediction {
    pub name: String,
    pub inbound: CountdownList,
    pub outbound: CountdownList,
}

#[derive(Debug)]
pub struct CountdownList(Vec<Countdown>);

/// Convert a list of timestamps into relative offsets from now, sorting and
/// truncating as necessary
impl From<Vec<DateTime<Utc>>> for CountdownList {
    fn from(value: Vec<DateTime<Utc>>) -> Self {
        let now = Utc::now();
        let countdowns = value
            .into_iter()
            // Get the first n upcoming timestamps
            .sorted()
            .take(Transit::MAX_PREDICTIONS)
            .map(|dt| Countdown((dt - now).num_minutes()))
            .collect();
        Self(countdowns)
    }
}

impl Display for CountdownList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().format(", "))
    }
}

/// Number of minutes until an event
#[derive(Debug)]
pub struct Countdown(i64);

impl Display for Countdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}m", self.0)
    }
}

/// <https://api-v3.mbta.com/docs/swagger/index.html#/Prediction/ApiWeb_PredictionController_index>
#[derive(Clone, Debug, Deserialize)]
struct ApiPredictions {
    data: Vec<Prediction>,
}

#[derive(Clone, Debug, Deserialize)]
struct Prediction {
    attributes: Attributes,
    relationships: Relationships,
}

#[derive(Clone, Debug, Deserialize)]
struct Attributes {
    departure_time: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize)]
struct Relationships {
    route: Relationship,
    stop: Relationship,
}

#[derive(Clone, Debug, Deserialize)]
struct Relationship {
    data: RelationshipData,
}

#[derive(Clone, Debug, Deserialize)]
struct RelationshipData {
    id: String,
}
