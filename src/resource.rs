//! A "resource" is a piece of hardware. Each resource has a submodule, which
//! implements all logic related to that resource.

pub mod lcd;

use log::info;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{sync::RwLock, time};

/// A hardware resource (e.g. LCD). This captures all generic logic for a
/// resource, including how to calculate and communicate hardware state. The
/// resource is responsible for communicating with the hardware to set its
/// state, as well as possibly maintaining its own internal state.
///
/// Each resource will have a separate async task spawned, which will run on a
/// fixed interval.
pub trait Resource: Sized {
    const INTERVAL: Duration = Duration::from_millis(100);

    /// Type of state managed by the user/API
    type UserState: 'static
        + Clone
        + Send
        + Sync
        + Serialize
        + Deserialize<'static>;

    /// Run a loop that will update hardware on a regular interva.
    async fn run(
        mut self,
        user_state: &RwLock<Self::UserState>,
    ) -> anyhow::Result<()> {
        info!("Starting resource {}", self.name());
        let mut interval = time::interval(Self::INTERVAL);
        self.on_start()?;
        loop {
            // Technically we're grabbing this read lock for longer than we may
            // need it. The alternative is to pass the RwLock down, which would
            // make it possible to modify user state, which is ugly. The call
            // should be fast enough that it's not an issue.
            self.on_tick(&*user_state.read().await)?;
            interval.tick().await;
        }
    }

    /// Get a descriptive name for this resource, for logging
    fn name(&self) -> &str;

    /// Update resource, once on startup
    fn on_start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Update hardware state on a fixed interval, based on the current user
    /// state. This call will hold a lock to the user state, so make it fast!
    fn on_tick(&mut self, user_state: &Self::UserState) -> anyhow::Result<()>;
}
