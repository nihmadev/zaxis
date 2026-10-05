//! Which addresses may be opened. Link text can come from anywhere, so the library never
//! hands an address to the system without checking its scheme against an allowlist.

use super::launch::launch;
use std::fmt;

/// Schemes a link may open. `https`, `http` and `mailto` are allowed by default; anything
/// else has to be added on purpose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlPolicy {
    schemes: Vec<String>,
}

impl Default for UrlPolicy {
    fn default() -> Self {
        Self {
            schemes: ["https", "http", "mailto"].map(String::from).to_vec(),
        }
    }
}

/// Why an address was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlError {
    /// No scheme, or characters that cannot be part of an address (whitespace, controls).
    Malformed,
    /// A well-formed scheme that is not on the allowlist.
    Scheme(String),
    /// The system could not start the handler.
    Open(String),
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("malformed address"),
            Self::Scheme(scheme) => write!(f, "scheme `{scheme}` is not allowed"),
            Self::Open(error) => write!(f, "could not open the address: {error}"),
        }
    }
}

impl std::error::Error for UrlError {}

impl UrlPolicy {
    /// Also allow `scheme` (letters, digits, `+`, `-`, `.`; case-insensitive).
    pub fn allow(mut self, scheme: impl AsRef<str>) -> Self {
        let scheme = scheme.as_ref().to_ascii_lowercase();
        if valid_scheme(&scheme) && !self.schemes.contains(&scheme) {
            self.schemes.push(scheme);
        }
        self
    }

    /// Only the given schemes.
    pub fn only<S: AsRef<str>>(schemes: impl IntoIterator<Item = S>) -> Self {
        schemes.into_iter().fold(
            Self {
                schemes: Vec::new(),
            },
            |policy, s| policy.allow(s),
        )
    }

    /// `url` unchanged when it is well-formed and its scheme is allowed. Leading or trailing
    /// whitespace, control characters and a missing scheme are refused, not repaired.
    pub fn check<'a>(&self, url: &'a str) -> Result<&'a str, UrlError> {
        if url.is_empty() || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(UrlError::Malformed);
        }
        let (scheme, rest) = url.split_once(':').ok_or(UrlError::Malformed)?;
        if !valid_scheme(scheme) || rest.is_empty() {
            return Err(UrlError::Malformed);
        }
        let scheme = scheme.to_ascii_lowercase();
        if self.schemes.contains(&scheme) {
            Ok(url)
        } else {
            Err(UrlError::Scheme(scheme))
        }
    }

    /// Open `url` with the system handler after [`Self::check`]. Blocks only until the
    /// handler has been started.
    pub fn open(&self, url: &str) -> Result<(), UrlError> {
        let url = self.check(url)?;
        launch(url).map_err(|e| UrlError::Open(e.to_string()))
    }
}

fn valid_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}
