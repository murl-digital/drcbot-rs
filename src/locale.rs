use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug)]
pub struct Translator {
    locales: HashMap<String, Locale>,
}

#[derive(Debug)]
pub enum InitError {
    IO(std::io::Error),
    Parse(toml::de::Error),
}

#[derive(Debug)]
pub enum GetError {
    NoMessage,
    NoLocale,
}

impl Translator {
    pub async fn new(path: &str) -> Result<Translator, InitError> {
        let file = match tokio::fs::read_to_string(path).await {
            Ok(s) => s,
            Err(why) => return Err(InitError::IO(why)),
        };

        let locales = match toml::from_str(&file) {
            Ok(l) => l,
            Err(why) => return Err(InitError::Parse(why)),
        };

        Ok(Translator { locales })
    }

    pub fn get(&self, key: &str, locale: &str) -> Result<String, GetError> {
        if let Some(message) = self.locales.get(key) {
            match locale {
                "en-US" => Ok(message.en_us.to_owned()),
                "en-GB" => Ok(message.en_uk.to_owned()),
                _ => Err(GetError::NoLocale),
            }
        } else {
            Err(GetError::NoMessage)
        }
    }
}

#[derive(Debug, Deserialize)]
struct Locale {
    en_us: String,
    en_uk: String,
}
