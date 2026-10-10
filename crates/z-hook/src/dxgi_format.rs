//! Which backbuffer formats the overlay can draw on, and how, for the DXGI paths. Plain numbers
//! (`DXGI_FORMAT` values), so it is the same on every platform and tested everywhere.

use crate::OutputColor;
use zaxis::wgpu::TextureFormat;

/// `DXGI_FORMAT` values of the formats this crate draws on.
pub mod dxgi {
    pub const R16G16B16A16_TYPELESS: u32 = 9;
    pub const R16G16B16A16_FLOAT: u32 = 10;
    pub const R10G10B10A2_TYPELESS: u32 = 23;
    pub const R10G10B10A2_UNORM: u32 = 24;
    pub const R8G8B8A8_TYPELESS: u32 = 27;
    pub const R8G8B8A8_UNORM: u32 = 28;
    pub const R8G8B8A8_UNORM_SRGB: u32 = 29;
    pub const B8G8R8A8_UNORM: u32 = 87;
    pub const B8G8R8A8_TYPELESS: u32 = 90;
    pub const B8G8R8A8_UNORM_SRGB: u32 = 91;
}

/// How to draw on a backbuffer of one format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatPlan {
    /// The family the shared texture is created in, so copies to and from the backbuffer are
    /// between compatible formats.
    pub typeless: u32,
    /// The format wgpu knows the texture as.
    pub texture: TextureFormat,
    /// The format of the view drawn through (the sRGB sibling of an 8 bit format).
    pub view: TextureFormat,
}

/// The plan for a backbuffer of DXGI format `format`, or why it cannot be drawn on.
pub fn plan(format: u32, output: OutputColor) -> Result<FormatPlan, String> {
    use dxgi::*;
    let (typeless, texture, float) = match format {
        R8G8B8A8_UNORM | R8G8B8A8_UNORM_SRGB | R8G8B8A8_TYPELESS => {
            (R8G8B8A8_TYPELESS, TextureFormat::Rgba8Unorm, false)
        }
        B8G8R8A8_UNORM | B8G8R8A8_UNORM_SRGB | B8G8R8A8_TYPELESS => {
            (B8G8R8A8_TYPELESS, TextureFormat::Bgra8Unorm, false)
        }
        R10G10B10A2_UNORM | R10G10B10A2_TYPELESS => {
            (R10G10B10A2_TYPELESS, TextureFormat::Rgb10a2Unorm, false)
        }
        R16G16B16A16_FLOAT | R16G16B16A16_TYPELESS => {
            (R16G16B16A16_TYPELESS, TextureFormat::Rgba16Float, true)
        }
        other => return Err(format!("backbuffer format {other} cannot be drawn on")),
    };
    match (output, float) {
        (OutputColor::Srgb, true) => {
            return Err("a float backbuffer holds linear values, not sRGB".into())
        }
        (OutputColor::Linear, false) => {
            return Err("only a float backbuffer can hold linear values".into())
        }
        _ => {}
    }
    Ok(FormatPlan {
        typeless,
        texture,
        view: texture.add_srgb_suffix(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_formats_draw_through_their_srgb_sibling() {
        let rgba = plan(dxgi::R8G8B8A8_UNORM, OutputColor::Auto).unwrap();
        assert_eq!(
            (rgba.texture, rgba.view),
            (TextureFormat::Rgba8Unorm, TextureFormat::Rgba8UnormSrgb)
        );
        assert_eq!(rgba.typeless, dxgi::R8G8B8A8_TYPELESS);
        let srgb = plan(dxgi::B8G8R8A8_UNORM_SRGB, OutputColor::Auto).unwrap();
        assert_eq!(
            (srgb.texture, srgb.view),
            (TextureFormat::Bgra8Unorm, TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn float_and_ten_bit_formats_are_drawn_as_they_are() {
        let float = plan(dxgi::R16G16B16A16_FLOAT, OutputColor::Auto).unwrap();
        assert_eq!(float.view, TextureFormat::Rgba16Float);
        let ten = plan(dxgi::R10G10B10A2_UNORM, OutputColor::Auto).unwrap();
        assert_eq!(ten.view, TextureFormat::Rgb10a2Unorm);
    }

    #[test]
    fn unusable_formats_and_color_requests_are_refused_with_a_reason() {
        assert!(
            plan(88, OutputColor::Auto).is_err(),
            "B8G8R8X8 has no alpha channel to blend into"
        );
        assert!(plan(dxgi::R16G16B16A16_FLOAT, OutputColor::Srgb).is_err());
        assert!(plan(dxgi::R8G8B8A8_UNORM, OutputColor::Linear).is_err());
        assert!(plan(dxgi::R16G16B16A16_FLOAT, OutputColor::Linear).is_ok());
    }
}
