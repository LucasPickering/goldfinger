/// User-facing state, modifiable by user input
#[derive(Copy, Clone, Debug, Default)]
pub struct UserState {
    pub mode: Mode,
    pub weather_period: usize,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Mode {
    #[default]
    Weather,
}
