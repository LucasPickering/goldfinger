//! A "resource" is a piece of hardware. Each resource has a submodule, which
//! implements all logic related to that resource.

pub mod lcd;

use crate::state::{LcdUserState, UserStateManager};
use log::{error, info};
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time};

/// A hardware resource (e.g. LCD). This captures all generic logic for a
/// resource, including how to calculate and communicate hardware state. The
/// resource is responsible for communicating with the hardware to set its
/// state, as well as possibly maintaining its own internal state.
///
/// Each resource will have a separate async task spawned, which will run on a
/// fixed interval.
///
/// This is overkill when we only have the LCD, but I copied it from Söze just
/// in case we need a second. It's only half-abstracted though so you'll need
/// to factor out some stuff around user state to add another resource type.
pub trait Resource: 'static + Send + Sized {
    const INTERVAL: Duration = Duration::from_millis(100);

    /// Run a loop that will update hardware on a regular interva.
    fn spawn_task(
        mut self,
        user_state: Arc<UserStateManager>,
    ) -> JoinHandle<()> {
        info!("Starting resource {}", self.name());
        tokio::spawn(async move {
            // Shitty try block
            let _result: anyhow::Result<()> = async {
                let mut interval = time::interval(Self::INTERVAL);
                self.on_start()?;
                loop {
                    let result = self.on_tick(*user_state.read().await);
                    if let Err(err) = result {
                        error!("Resource {} failed with {}", self.name(), err);
                    }
                    interval.tick().await;
                }
            }
            .await;
        })
    }

    /// Get a descriptive name for this resource, for logging
    fn name(&self) -> &str;

    /// Update resource, once on startup
    fn on_start(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Update hardware state on a fixed interval, based on the current user
    /// state
    fn on_tick(&mut self, user_state: LcdUserState) -> anyhow::Result<()>;
}
