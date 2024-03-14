use reqwest::header::InvalidHeaderValue;
use std::string::FromUtf8Error;
use url::ParseError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    HeaderError(#[from] InvalidHeaderValue),
    #[error(transparent)]
    BuildError(#[from] reqwest::Error),
    #[error(transparent)]
    UrlBuilder(#[from] url_builder::Error),
    #[error("Token not found")]
    TokenNotFound,
    #[error("generic")]
    Generic,
    #[error(transparent)]
    Utf8ParseError(#[from] FromUtf8Error),
    #[error(transparent)]
    UrlParseError(#[from] ParseError),
    #[error("Could not find `code` in response.")]
    CodeNotFound,
}

