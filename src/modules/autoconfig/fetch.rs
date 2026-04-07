#![allow(dead_code)] // Mirrors upstream autoconfig public API surface.

//
// Mozilla Thunderbird autoconfig discovery over HTTP(S), using reqwest (rustls) instead of libcurl.
// Logic mirrors the MIT-licensed `autoconfig` crate:
// https://github.com/Dust-Mail/autoconfig (v0.4.0, src/lib.rs, src/client.rs, src/dns.rs, src/http.rs, src/parse.rs, src/utils.rs)
//

use std::{io, time::Duration};

use bytes::Bytes;
use futures::{future::select_ok, FutureExt};
use regex::Regex;
use reqwest::Client;
use trust_dns_resolver::{
    config::ResolverConfig,
    error::ResolveError,
    TokioAsyncResolver,
};

use super::mozilla_config::Config;

const AT_SYMBOL: char = '@';

const EMAIL_REGEX: &str =
    r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})";

#[derive(Debug)]
pub enum ErrorKind {
    Http(reqwest::Error),
    BuildHttpClient,
    InvalidResponse,
    BadInput,
    Resolve(ResolveError),
    NotFound(Vec<Error>),
    ParseXml(serde_xml_rs::Error),
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new<S: Into<String>>(kind: ErrorKind, msg: S) -> Self {
        Self {
            kind,
            message: msg.into(),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::new(ErrorKind::Http(error), "HTTP request failed")
    }
}

impl From<serde_xml_rs::Error> for Error {
    fn from(error: serde_xml_rs::Error) -> Self {
        Self::new(ErrorKind::ParseXml(error), "Error parsing XML response")
    }
}

impl From<ResolveError> for Error {
    fn from(error: ResolveError) -> Self {
        Self::new(ErrorKind::Resolve(error), "Error resolving DNS")
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn validate_email(unknown_str: &str) -> bool {
    let email_regex = Regex::new(EMAIL_REGEX).unwrap();
    email_regex.is_match(unknown_str)
}

fn parse_config(bytes: &[u8]) -> Result<Config> {
    let reader = io::Cursor::new(bytes);
    let config: Config = serde_xml_rs::from_reader(reader)?;
    Ok(config)
}

struct AutoconfigClient {
    http: Client,
    resolver: TokioAsyncResolver,
}

impl AutoconfigClient {
    async fn new() -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .use_rustls_tls()
            .build()
            .map_err(|_| Error::new(ErrorKind::BuildHttpClient, "Failed to create HTTP client"))?;
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), Default::default())?;
        Ok(Self { http, resolver })
    }

    const TXT_RECORD_REGEX: &str = r"^mailconf=(https?://\S+)$";

    async fn get_url_from_txt<N: AsRef<str>>(&self, name: N) -> Result<Vec<String>> {
        let lookup_results = self.resolver.txt_lookup(name.as_ref()).await?;
        let re = Regex::new(Self::TXT_RECORD_REGEX).unwrap();
        let mut urls = Vec::new();
        for txt in lookup_results {
            let mut chunks: Vec<Bytes> = txt
                .txt_data()
                .iter()
                .map(|data| data.to_vec().into())
                .collect();
            if chunks.first().is_some() {
                let record = chunks.remove(0);
                if let Some(record_str) = std::str::from_utf8(&record).ok() {
                    if let Some(captured) = re.captures(record_str) {
                        if let Some(m) = captured.get(1) {
                            let url = m.as_str();
                            if let Ok(url_parsed) = url::Url::parse(url) {
                                if url_parsed.scheme() == "https" {
                                    urls.push(url.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(urls)
    }
}

/// Given an email provider domain, fetch Thunderbird autoconfig XML.
pub async fn from_domain<D: AsRef<str>>(domain: D) -> Result<Config> {
    let mut errors: Vec<Error> = Vec::new();
    let client = AutoconfigClient::new().await?;

    let mut urls = vec![
        format!("http://autoconfig.{}/mail/config-v1.1.xml", domain.as_ref()),
        format!(
            "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
            domain.as_ref()
        ),
        format!(
            "https://autoconfig.thunderbird.net/v1.1/{}",
            domain.as_ref()
        ),
    ];

    match client.get_url_from_txt(domain.as_ref()).await {
        Ok(txt_urls) => {
            for url in txt_urls {
                urls.push(url);
            }
        }
        Err(error) => errors.push(error),
    }

    urls.sort();
    urls.dedup();

    let http = client.http.clone();
    let mut futures = Vec::new();
    for url in urls {
        let http = http.clone();
        futures.push(
            async move {
                let response = http
                    .get(&url)
                    .send()
                    .await
                    .map_err(Error::from)?;
                let status = response.status();
                let bytes = response.bytes().await.map_err(Error::from)?;
                if !status.is_success() {
                    return Err(Error::new(
                        ErrorKind::InvalidResponse,
                        format!(
                            "HTTP request failed: {}",
                            String::from_utf8_lossy(bytes.as_ref())
                        ),
                    ));
                }
                parse_config(bytes.as_ref())
            }
            .boxed(),
        );
    }

    match select_ok(futures).await {
        Ok((config, _remaining)) => Ok(config),
        Err(error) => {
            errors.push(error);
            Err(Error::new(
                ErrorKind::NotFound(errors),
                "Could not find a valid config",
            ))
        }
    }
}

/// Given an email address, resolve autoconfig for its domain.
pub async fn from_addr(email_address: &str) -> Result<Config> {
    if !validate_email(email_address) {
        return Err(Error::new(ErrorKind::BadInput, "Given email address is invalid"));
    }
    let mut split = email_address.split(AT_SYMBOL);
    split.next();
    let domain = match split.next() {
        Some(domain) => domain,
        None => {
            return Err(Error::new(
                ErrorKind::BadInput,
                "An email address must specify a domain after the '@' symbol",
            ))
        }
    };
    from_domain(domain).await
}
