use crate::{config::Config, state::UserState};
use std::time::Duration;

/// Mock display, to allow compiling/running tests on non-Linux machines
pub struct Display;

impl Display {
    pub const INTERVAL: Duration = Duration::from_millis(1000);

    pub fn new(_: &Config) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn tick(&mut self, _: &UserState) -> anyhow::Result<()> {
        Ok(())
    }
}
