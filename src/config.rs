use std::time::Duration;

use confy::ConfyError;
use serde::{Deserialize, Serialize};
use tokio::time;
use tracing::*;

pub const CONFIG_NAME: &str = "save-it";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub language: String,
    pub format_standard: FormatStandard,
    pub custom_format: String,
}

impl Default for Config {
    fn default() -> Self {
        trace!("Creating new config");

        Self {
            language: "en".to_string(),
            format_standard: FormatStandard::Default,
            custom_format: "CUSTOM FORMAT".to_string(),
        }
    }
}

impl Config {
    pub fn get_config() -> Self {
        debug!("Getting config");

        let res: Result<Config, ConfyError> = confy::load(CONFIG_NAME, None);

        res.unwrap_or_else(|e| {
            if let ConfyError::BadTomlData(_) = e {
                let default = Config::default();

                confy::store(CONFIG_NAME, None, default).expect("Error resetting config");
                Self::get_config()
            } else {
                panic!("Error loading config: {}", &e)
            }
        })
    }

    pub fn save(&self) {
        debug!("Saving config");
        let config = self.clone();

        tokio::task::spawn(async move {
            confy::store(CONFIG_NAME, None, config).expect("Error saving config");
        });
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Copy)]
pub enum FormatStandard {
    Default,
    Custom,
}
