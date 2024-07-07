use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug)]
pub struct Translator {
    locales: HashMap<String, Locale>,
}

#[derive(Error, Debug)]
pub enum InitError {
    #[error("couldn't read io from disk: {0}")]
    IO(std::io::Error),
    #[error("couldn't parse locale file: {0}")]
    Parse(toml::de::Error),
}

#[derive(Debug)]
pub enum GetError {
    NoMessage,
    NoLocale,
}

impl Translator {
    pub async fn new(path: &str) -> Result<Self, InitError> {
        let file = match tokio::fs::read_to_string(path).await {
            Ok(s) => s,
            Err(why) => return Err(InitError::IO(why)),
        };

        let locales = match toml::from_str(&file) {
            Ok(l) => l,
            Err(why) => return Err(InitError::Parse(why)),
        };

        Ok(Self { locales })
    }

    pub fn get(&self, key: &str, locale: &str) -> Result<String, GetError> {
        self.locales
            .get(key)
            .map_or(Err(GetError::NoMessage), |message| match locale {
                "en-US" => Ok(message.en_us.clone()),
                "en-GB" => Ok(message.en_uk.clone()),
                _ => Err(GetError::NoLocale),
            })
    }
}

#[derive(Debug, Deserialize)]
struct Locale {
    en_us: String,
    en_uk: String,
}
