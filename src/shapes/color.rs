/// Straight-alpha, sRGB color. The renderer converts RGB to linear before blending.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const TRANSPARENT: Self = Self([0, 0, 0, 0]);
    pub const WHITE: Self = Self([255; 4]);
    pub const BLACK: Self = Self([0, 0, 0, 255]);
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }
    /// The same color with a replaced alpha channel.
    ///
    /// ```
    /// use zaxis::Color;
    /// assert_eq!(Color::rgb(10, 20, 30).with_alpha(128), Color::rgba(10, 20, 30, 128));
    /// ```
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self([self.0[0], self.0[1], self.0[2], alpha])
    }

    /// The same color with its alpha channel scaled by `opacity` (clamped to 0–1).
    ///
    /// ```
    /// use zaxis::Color;
    /// assert_eq!(Color::rgba(1, 2, 3, 200).with_opacity(0.5).0[3], 100);
    /// ```
    pub fn with_opacity(self, opacity: f32) -> Self {
        let alpha = f32::from(self.0[3]) * opacity.clamp(0.0, 1.0);
        self.with_alpha(alpha.round() as u8)
    }

    /// Creates an opaque color from `0xRRGGBB`.
    ///
    /// ```
    /// use zaxis::Color;
    /// const ACCENT: Color = Color::hex(0x4E85BE);
    /// assert_eq!(ACCENT, Color::rgb(78, 133, 190));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `rgb` exceeds `0xFFFFFF`.
    pub const fn hex(rgb: u32) -> Self {
        assert!(rgb <= 0xFFFFFF, "HEX RGB color must fit in 24 bits");
        Self::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Parses RGB or RGBA hex digits, optionally prefixed by `#`.
    ///
    /// Accepts `RGB`, `RGBA`, `RRGGBB`, and `RRGGBBAA`, in either case.
    /// Short forms duplicate each digit; omitted alpha is 255. Returns `None`
    /// for invalid digits or lengths. Whitespace and `0x` prefixes are rejected.
    ///
    /// ```
    /// use zaxis::Color;
    /// assert_eq!(Color::from_hex("#4E85BE"), Some(Color::hex(0x4E85BE)));
    /// assert_eq!(Color::from_hex("#4E85BE80"), Some(Color::rgba(78, 133, 190, 128)));
    /// assert_eq!(Color::from_hex("invalid"), None);
    /// ```
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.strip_prefix('#').unwrap_or(hex).as_bytes();
        let (channels, short) = match digits.len() {
            3 => (3, true),
            4 => (4, true),
            6 => (3, false),
            8 => (4, false),
            _ => return None,
        };
        let nibble = |digit| match digit {
            b'0'..=b'9' => Some(digit - b'0'),
            b'a'..=b'f' => Some(digit - b'a' + 10),
            b'A'..=b'F' => Some(digit - b'A' + 10),
            _ => None,
        };
        let mut rgba = [0, 0, 0, 255];
        for (index, channel) in rgba.iter_mut().take(channels).enumerate() {
            *channel = if short {
                nibble(digits[index])? * 17
            } else {
                nibble(digits[index * 2])? * 16 + nibble(digits[index * 2 + 1])?
            };
        }
        Some(Self(rgba))
    }

    pub const fn gray(value: u8) -> Self {
        Self::rgb(value, value, value)
    }

    pub fn linear(self) -> [f32; 4] {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        [
            channel(self.0[0]),
            channel(self.0[1]),
            channel(self.0[2]),
            f32::from(self.0[3]) / 255.0,
        ]
    }
}
