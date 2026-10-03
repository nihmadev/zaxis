//! Renderer failures.

use std::{error::Error, fmt};

/// Initialization, device failure, or invalid drawing data.
#[derive(Debug)]
pub enum RenderError {
    Texture(String),
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    UnsupportedSurface,
    DeviceLost(String),
    Validation(String),
    InvalidDrawData(&'static str),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Texture(message) => write!(f, "image texture: {message}"),
            Self::CreateSurface(error) => write!(f, "creating the GPU surface: {error}"),
            Self::RequestAdapter(error) => write!(f, "finding a GPU adapter: {error}"),
            Self::RequestDevice(error) => write!(f, "opening a GPU device: {error}"),
            Self::UnsupportedSurface => f.write_str("the adapter does not support this surface"),
            Self::DeviceLost(message) => write!(f, "GPU device lost: {message}"),
            Self::Validation(message) => write!(f, "GPU validation failed: {message}"),
            Self::InvalidDrawData(message) => write!(f, "invalid UI draw data: {message}"),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            _ => None,
        }
    }
}
