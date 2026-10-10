use crate::Icon;
use core::fmt::{self, Write};

/// An SVG document with configurable root attributes, formatted without allocation.
///
/// Use `to_string()` in applications with an allocator, or [`Self::write_to`] with a
/// fixed buffer. Size and stroke width initially preserve the original document;
/// stroke color defaults to `currentColor`. The viewBox and geometry are preserved.
#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct Svg<'a> {
    icon: &'a Icon,
    color: &'a str,
    size: Option<[u32; 2]>,
    stroke_width: Option<f32>,
}

impl<'a> Svg<'a> {
    pub(crate) const fn new(icon: &'a Icon) -> Self {
        Self {
            icon,
            color: "currentColor",
            size: None,
            stroke_width: None,
        }
    }

    /// Set a CSS/SVG color for the root stroke and bundled filled dots.
    /// XML attribute characters are escaped.
    ///
    /// Use `currentColor` for inline SVG; CSS color inheritance does not cross an
    /// external image boundary. CSS values are passed through, not validated.
    pub const fn color(mut self, color: &'a str) -> Self {
        self.color = color;
        self
    }

    /// Set intrinsic width and height in pixels, preserving the original viewBox.
    ///
    /// # Panics
    /// Panics if either dimension is zero.
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "SVG dimensions must be positive");
        self.size = Some([width, height]);
        self
    }

    /// Set root stroke width in viewBox units (Lucide's default is 2).
    ///
    /// # Panics
    /// Panics if the value is negative or not finite. Zero hides the stroke.
    pub fn stroke_width(mut self, width: f32) -> Self {
        assert!(width.is_finite() && width >= 0.0, "invalid stroke width");
        self.stroke_width = Some(width);
        self
    }

    /// Write SVG to a caller-owned buffer, without allocating.
    ///
    /// Returns the writer's error, or [`fmt::Error`] for a malformed custom root.
    pub fn write_to(&self, output: &mut impl Write) -> fmt::Result {
        write!(output, "{self}")
    }
}

impl fmt::Display for Svg<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = self.icon.svg_str().trim_start();
        let mut attrs = source.strip_prefix("<svg").ok_or(fmt::Error)?;
        if !attrs.starts_with(char::is_whitespace) && !attrs.starts_with(['>', '/']) {
            return Err(fmt::Error);
        }
        output.write_str("<svg stroke=\"")?;
        escape(self.color, output)?;
        output.write_char('"')?;
        if let Some([width, height]) = self.size {
            write!(output, " width=\"{width}\" height=\"{height}\"")?;
        }
        if let Some(width) = self.stroke_width {
            write!(output, " stroke-width=\"{width}\"")?;
        }
        loop {
            attrs = attrs.trim_start();
            if attrs.starts_with('>') || attrs.starts_with("/>") {
                if !self.icon.bundled {
                    return output.write_str(attrs);
                }
                // The bundled snapshot is normalized to white, including filled dots.
                // Recolor all of those paints; custom documents retain child overrides.
                let mut parts = attrs.split("\"#fff\"");
                output.write_str(parts.next().unwrap_or_default())?;
                for part in parts {
                    output.write_char('"')?;
                    escape(self.color, output)?;
                    output.write_char('"')?;
                    output.write_str(part)?;
                }
                return Ok(());
            }
            let end = attrs
                .find(|c: char| c.is_whitespace() || c == '=')
                .ok_or(fmt::Error)?;
            let name = &attrs[..end];
            if name.is_empty() {
                return Err(fmt::Error);
            }
            let value = attrs[end..]
                .trim_start()
                .strip_prefix('=')
                .ok_or(fmt::Error)?
                .trim_start();
            let quote = value.chars().next().ok_or(fmt::Error)?;
            if quote != '\'' && quote != '"' {
                return Err(fmt::Error);
            }
            let value_end = value[1..].find(quote).ok_or(fmt::Error)? + 2;
            let keep = match name {
                "stroke" => false,
                "width" | "height" => self.size.is_none(),
                "stroke-width" => self.stroke_width.is_none(),
                _ => true,
            };
            if keep {
                write!(output, " {name}={}", &value[..value_end])?;
            }
            attrs = &value[value_end..];
        }
    }
}

fn escape(value: &str, output: &mut impl Write) -> fmt::Result {
    for ch in value.chars() {
        match ch {
            '&' => output.write_str("&amp;")?,
            '<' => output.write_str("&lt;")?,
            '>' => output.write_str("&gt;")?,
            '"' => output.write_str("&quot;")?,
            '\'' => output.write_str("&apos;")?,
            _ => output.write_char(ch)?,
        }
    }
    Ok(())
}
