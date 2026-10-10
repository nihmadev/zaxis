//! Why installing or running the overlay failed.

use std::{error::Error, fmt};

/// A failure to install the overlay. Failures while it runs never reach the host: they disable
/// the overlay and are logged (see [`Overlay::disabled_reason`](crate::Overlay::disabled_reason)).
#[derive(Debug)]
#[non_exhaustive]
pub enum HookError {
    /// An overlay is installed already; there is one per process.
    AlreadyInstalled,
    /// None of the requested APIs can be hooked on this platform or build.
    NoSupportedApi,
    /// The options ask for something that cannot work.
    InvalidOptions(String),
    /// A hook could not be placed.
    Hook(String),
}

impl fmt::Display for HookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInstalled => {
                f.write_str("an overlay is already installed in this process")
            }
            Self::NoSupportedApi => {
                f.write_str("none of the requested graphics APIs can be hooked here")
            }
            Self::InvalidOptions(message) => write!(f, "invalid overlay options: {message}"),
            Self::Hook(message) => write!(f, "could not place the hook: {message}"),
        }
    }
}

impl Error for HookError {}
