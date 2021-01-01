use failure::Fail;
use url;

#[derive(Debug, Fail)]
pub enum ManifestError {
  #[fail(display = "IO Error: {}", e)]
  IO { e: std::io::Error },

  #[fail(display = "Url Parse Error: {}", e)]
  UrlParseError { e: url::ParseError },

  #[fail(display = "Reqwest Error: {}", e)]
  ReqwestError { e: reqwest::Error },

  #[fail(display = "Serde JSON Error: {}", e)]
  SerdeJsonError { e: serde_json::Error },

  #[fail(display = "None Error: {:?}", e)]
  NoneError { e: std::option::NoneError },

  #[fail(display = "Serde YAML Error: {}", e)]
  SerdeYamlError { e: serde_yaml::Error },
}

impl From<std::io::Error> for ManifestError {
  fn from(e: std::io::Error) -> Self {
    ManifestError::IO { e }
  }
}

impl From<url::ParseError> for ManifestError {
  fn from(e: url::ParseError) -> Self {
    ManifestError::UrlParseError { e }
  }
}

impl From<reqwest::Error> for ManifestError {
  fn from(e: reqwest::Error) -> Self {
    ManifestError::ReqwestError { e }
  }
}

impl From<serde_json::Error> for ManifestError {
    fn from(e: serde_json::Error) -> Self {
        ManifestError::SerdeJsonError { e }
    }
}

impl From<std::option::NoneError> for ManifestError {
    fn from(e: std::option::NoneError) -> Self {
        ManifestError::NoneError { e }
    }
}

impl From<serde_yaml::Error> for ManifestError {
    fn from(e: serde_yaml::Error) -> Self {
        ManifestError::SerdeYamlError { e }
    }
}
