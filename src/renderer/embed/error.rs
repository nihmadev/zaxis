//! Why embedding failed.

use crate::RenderError;
use std::{error::Error, fmt};

/// A failure of an [`EmbeddedRenderer`](super::EmbeddedRenderer). Every case leaves the
/// renderer and the host's device untouched: the host decides whether to skip the frame,
/// change its target or recreate the renderer.
#[derive(Debug)]
pub enum EmbedError {
    /// The host reported the device lost (see
    /// [`notify_device_lost`](super::EmbeddedRenderer::notify_device_lost)). Make a new renderer
    /// on the new device; draw data carries everything it needs.
    DeviceLost(String),
    /// The options name a format, sample count or depth format the renderer cannot draw to.
    InvalidOptions(&'static str),
    /// The region does not fit the target or the draw data was built for another size.
    InvalidViewport(String),
    /// The target texture lacks a usage the frame needs.
    TargetUsage {
        missing: wgpu::TextureUsages,
        reason: &'static str,
    },
    /// The target texture differs from what the options describe.
    TargetMismatch(String),
    /// `record` was called before `prepare`, or the options changed since.
    NotPrepared,
    /// The draw data, a texture upload or a material was rejected.
    Render(RenderError),
}

impl fmt::Display for EmbedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceLost(message) => write!(f, "GPU device lost: {message}"),
            Self::InvalidOptions(message) => write!(f, "invalid embedding options: {message}"),
            Self::InvalidViewport(message) => write!(f, "invalid embedding viewport: {message}"),
            Self::TargetUsage { missing, reason } => {
                write!(f, "the target texture needs {missing:?}: {reason}")
            }
            Self::TargetMismatch(message) => {
                write!(f, "target does not match the options: {message}")
            }
            Self::NotPrepared => f.write_str("record called without a prepared frame"),
            Self::Render(error) => error.fmt(f),
        }
    }
}

impl Error for EmbedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Render(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RenderError> for EmbedError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::DeviceLost(message) => Self::DeviceLost(message),
            error => Self::Render(error),
        }
    }
}
