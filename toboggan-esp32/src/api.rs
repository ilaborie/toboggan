use anyhow::{bail, Context};
use embedded_svc::http::client::Client;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::{debug, info};
use serde::Deserialize;

use crate::state::TalkData;

#[derive(Debug, Clone, Deserialize)]
struct Talk {
    pub title: String,
    pub titles: Vec<String>,
}

impl From<Talk> for TalkData {
    fn from(talk: Talk) -> Self {
        TalkData::new(talk.title, talk.titles)
    }
}

// Allow dead code for API module since it's currently unused but will be needed
#[allow(dead_code)]
pub struct Api {
    client: Client<EspHttpConnection>,
    base_url: String,
}

#[allow(dead_code)]
impl Api {
    pub(crate) fn new(base_url: String) -> anyhow::Result<Self> {
        let configuration = Configuration::default();
        let conn = EspHttpConnection::new(&configuration).context("creating the HTTP conection")?;
        let client = Client::wrap(conn);
        Ok(Self { client, base_url })
    }

    pub(crate) fn talk(&mut self) -> anyhow::Result<TalkData> {
        let uri = format!("{}/api/talk", self.base_url.trim_end_matches('/'));
        let request = self
            .client
            .get(&uri)
            .with_context(|| format!("build GET {uri} request"))?;
        let mut response = request.submit().with_context(|| format!("GET {uri}"))?;
        let status = response.status();
        info!("Status: [{status}]",);

        if !(200..300).contains(&status) {
            bail!("HTTP status not OK ({status})");
        }

        let mut result = vec![];
        let mut buf = [0_u8; 256];
        let mut total = 0;
        while let Ok(size) = response.read(&mut buf) {
            total += size;
            if size == 0 {
                break;
            }
            result.extend(buf.get(0..size).unwrap_or(&[]));
        }
        debug!("total len: {total}");

        let json_str = String::from_utf8(result).context("Invalid UTF8 response")?;
        debug!("JSON response: {json_str}");

        let talk: Talk =
            serde_json::from_str(&json_str).context("Failed to parse JSON response as Talk")?;

        Ok(talk.into())
    }
}
