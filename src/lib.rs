#![doc = include_str!("../README.md")]

use std::{str::FromStr, time::Duration};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use url::Url;

mod ident;

pub use ident::IdentificationMethod;

/// The interface for accessing a Nominatim API server.
#[derive(Debug, Clone)]
pub struct Client {
    ident: IdentificationMethod, // how to access the server
    base_url: Url,               // defaults to https://nominatim.openstreetmap.org
    client: reqwest::Client,
    /// HTTP Request Timeout [`Duration`]
    pub timeout: Duration,
}

impl Client {
    /// Create a new [`Client`] from an [`IdentificationMethod`].
    pub fn new(ident: IdentificationMethod) -> Self {
        let timeout = Duration::from_secs(10);

        Self {
            ident,
            base_url: Url::parse("https://nominatim.openstreetmap.org/").unwrap(),
            client: reqwest::ClientBuilder::new()
                .timeout(timeout)
                .build()
                .unwrap(),
            timeout,
        }
    }

    /// Set the client's internal base url for all requests.
    pub fn set_base_url<U: TryInto<Url>>(&mut self, url: U) -> Result<(), U::Error> {
        self.base_url = url.try_into()?;

        Ok(())
    }

    /// Check the status of the nominatim server.
    /// ```
    /// # use tag_nominatim::{Client, IdentificationMethod};
    ///
    /// let client = Client::new(IdentificationMethod::from_user_agent(
    ///     "Example Application Name",
    /// ));
    /// # tokio_test::block_on(async {
    /// assert_eq!(client.status().await.unwrap().message, "OK");
    /// # })
    /// ```
    pub async fn status(&self) -> Result<Status, reqwest::Error> {
        let mut url = self.base_url.join("status.php").unwrap();
        url.set_query(Some("format=json"));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Get [`Place`]s from a search query.
    ///
    /// ```
    /// # use tag_nominatim::{Client, IdentificationMethod};
    ///
    /// let client = Client::new(IdentificationMethod::from_user_agent(
    ///     "Example Application Name",
    /// ));
    /// # tokio_test::block_on(async {
    /// assert_eq!(client.search("statue of liberty").await.unwrap().len(), 4);
    /// # })
    /// ```
    pub async fn search(&self, query: impl Into<String>) -> Result<Vec<Place>, reqwest::Error> {
        let mut url = self.base_url.clone();
        url.set_query(Some(&format!(
            "addressdetails=1&extratags=1&q={}&format=json",
            query.into().replace(' ', "+")
        )));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Generate a [`Place`] from latitude and longitude.
    ///
    /// ```
    /// # use tag_nominatim::{Client, IdentificationMethod};
    ///
    /// let client = Client::new(IdentificationMethod::from_user_agent(
    ///     "Example Application Name",
    /// ));
    /// # tokio_test::block_on(async {
    /// assert_eq!(
    ///     client.reverse("40.689249", "-74.044500", None).await.unwrap().display_name,
    ///     "Statue of Liberty, Flagpole Plaza, Manhattan Community Board 1, Manhattan, New York County, City of New York, New York, 10004, United States"
    /// );
    /// # })
    /// ```
    pub async fn reverse(
        &self,
        latitude: impl Into<String>,
        longitude: impl Into<String>,
        zoom: Option<u8>,
    ) -> Result<Place, reqwest::Error> {
        let mut url = self.base_url.join("reverse").unwrap();

        match zoom {
            Some(zoom) => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}&zoom={}",
                    latitude.into().replace(' ', ""),
                    longitude.into().replace(' ', ""),
                    zoom
                )));
            }
            None => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}",
                    latitude.into().replace(' ', ""),
                    longitude.into().replace(' ', ""),
                )));
            }
        }

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Return [`Place`]s from a list of OSM Node, Way, or Relations.
    ///
    /// ```
    /// # use tag_nominatim::{Client, IdentificationMethod};
    ///
    /// let client = Client::new(IdentificationMethod::from_user_agent(
    ///     "Example Application Name",
    /// ));
    /// # tokio_test::block_on(async {
    /// assert_eq!(
    ///     client.lookup(vec!["R146656", "W50637691"]).await.unwrap().first().unwrap().display_name,
    ///     "Manchester, Greater Manchester, England, United Kingdom"
    /// );
    /// # })
    /// ```
    pub async fn lookup(
        &self,
        queries: Vec<impl Into<String>>,
    ) -> Result<Vec<Place>, reqwest::Error> {
        let queries: String = queries
            .into_iter()
            .map(Into::<String>::into)
            .collect::<Vec<String>>()
            .join(",");

        let mut url = self.base_url.join("lookup").unwrap();
        url.set_query(Some(&format!(
            "osm_ids={}&addressdetails=1&extratags=1&format=json",
            queries
        )));

        let mut headers = HeaderMap::new();
        headers.append(
            HeaderName::from_str(&self.ident.header()).expect("invalid nominatim auth header name"),
            HeaderValue::from_str(&self.ident.value())
                .expect("invalid nominatim auth header value"),
        );

        self.client
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }
}

/// The status of a Nominatim server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub status: usize,
    pub message: String,
    pub data_updated: Option<String>,
    pub software_version: Option<String>,
    pub database_version: Option<String>,
}

/// A location returned by the Nominatim server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Place {
    #[serde(default)]
    pub place_id: usize,
    #[serde(default)]
    pub licence: String,
    #[serde(default)]
    pub osm_type: String,
    #[serde(default)]
    pub osm_id: usize,
    #[serde(default)]
    pub boundingbox: Vec<String>,
    #[serde(default)]
    pub lat: String,
    #[serde(default)]
    pub lon: String,
    #[serde(default)]
    pub display_name: String,
    pub class: Option<String>,
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub importance: Option<f64>,
    pub icon: Option<String>,
    #[serde(default)]
    pub address: Option<Address>,
    pub extratags: Option<ExtraTags>,
}

/// An address for a place.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Address {
    pub city: Option<String>,
    pub state_district: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "ISO3166-2-lvl4")]
    pub iso3166_2_lvl4: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}

/// Extra metadata that a place may have.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraTags {
    pub capital: Option<String>,
    pub website: Option<String>,
    pub wikidata: Option<String>,
    pub wikipedia: Option<String>,
    pub population: Option<String>,
}
