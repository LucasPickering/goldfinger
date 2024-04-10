use anyhow::Context;
use log::{error, info};
use rocket::{FromForm, FromFormField};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tokio::{fs, sync::RwLock};

/// Wrapper for user state, that handles loading/saving
#[derive(Debug, Default)]
pub struct UserStateManager {
    state: RwLock<LcdUserState>,
}

impl UserStateManager {
    const FILE: &'static str = "./settings.json";

    pub async fn load() -> Self {
        // Shitty try block
        let helper = || async {
            let contents = fs::read(Self::FILE).await?;
            Ok::<LcdUserState, anyhow::Error>(serde_json::from_slice(
                &contents,
            )?)
        };
        match helper().await {
            Ok(state) => Self {
                state: RwLock::new(state),
            },
            Err(err) => {
                error!("Error loading user state from {}: {}", Self::FILE, err);
                Self::default()
            }
        }
    }

    pub async fn set(&self, new_state: LcdUserState) -> anyhow::Result<()> {
        *self.state.write().await = new_state;
        info!("Saving user state: {:?}", &new_state);
        let serialized = serde_json::to_string_pretty(&new_state)?;
        fs::write(Self::FILE, &serialized).await.with_context(|| {
            format!("Error saving user state to {}", Self::FILE)
        })?;
        Ok(())
    }

    pub async fn read(&self) -> impl '_ + Deref<Target = LcdUserState> {
        self.state.read().await
    }
}

/// User-facing LCD state
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, FromForm)]
pub struct LcdUserState {
    pub mode: LcdMode,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    FromFormField,
)]
#[serde(rename_all = "snake_case")]
pub enum LcdMode {
    #[default]
    Weather,
}
