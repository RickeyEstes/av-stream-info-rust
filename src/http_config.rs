use serde::de::{self, Deserialize, Deserializer, Unexpected};
use std::error::Error;

use reqwest::blocking::get;

#[derive(Serialize, Deserialize, Debug)]
pub struct MetaInfoFile {
    #[serde(rename = "icy-index-metadata")]
    #[serde(deserialize_with = "bool_from_int")]
    pub index_metadata: bool,
    #[serde(rename = "icy-version")]
    pub version: u8,
    #[serde(rename = "icy-main-stream-url")]
    pub main_stream_url: Option<String>,
    #[serde(rename = "icy-name")]
    pub name: Option<String>,
    #[serde(rename = "icy-description")]
    pub description: Option<String>,
    #[serde(rename = "icy-genre")]
    pub genre: Option<String>,
    #[serde(rename = "icy-language-codes")]
    pub languages: Option<String>,
    #[serde(rename = "icy-country-code")]
    pub countrycode: Option<String>,
    #[serde(rename = "icy-country-subdivision-code")]
    pub country_subdivision_code: Option<String>,
    #[serde(rename = "icy-logo")]
    pub logo: Option<String>,
}

pub fn extract_from_homepage(homepage: &str) -> Result<MetaInfoFile, Box<dyn Error>> {
    let stream_info_link = format!("{}/streaminfo.json", homepage);

    trace!(
        "extract_from_homepage({}) Download file '{}'",
        homepage,
        stream_info_link
    );
    let resp = get(&stream_info_link)?.text()?;
    let deserialized: MetaInfoFile = serde_json::from_str(&resp)?;
    Ok(deserialized)
}

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(de::Error::invalid_value(
            Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}
