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

    pub(crate) fn linear(self) -> [f32; 4] {
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

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn hex_integer_is_const_and_opaque() {
        const ACCENT: Color = Color::hex(0x4E85BE);
        assert_eq!(ACCENT, Color::rgb(78, 133, 190));
        assert_eq!(Color::hex(0), Color::BLACK);
        assert_eq!(Color::hex(0xFFFFFF), Color::WHITE);
        assert_eq!(Color::hex(0x000102), Color::rgb(0, 1, 2));
    }

    #[test]
    #[should_panic(expected = "HEX RGB color must fit in 24 bits")]
    fn hex_integer_rejects_extra_bits() {
        Color::hex(0x1000000);
    }

    #[test]
    fn hex_strings_accept_rgb_and_rgba() {
        for hex in ["#4E85BE", "4e85be", "#4e85BE"] {
            assert_eq!(Color::from_hex(hex), Some(Color::rgb(78, 133, 190)));
        }
        for hex in ["#4E85BE80", "4e85be80"] {
            assert_eq!(Color::from_hex(hex), Some(Color::rgba(78, 133, 190, 128)));
        }
        for hex in ["#aBc", "ABC"] {
            assert_eq!(Color::from_hex(hex), Some(Color::rgb(170, 187, 204)));
        }
        for hex in ["#aBc8", "ABC8"] {
            assert_eq!(Color::from_hex(hex), Some(Color::rgba(170, 187, 204, 136)));
        }
        assert_eq!(Color::from_hex("00000000"), Some(Color::TRANSPARENT));
        assert_eq!(Color::from_hex("ffffffff"), Some(Color::WHITE));
    }

    #[test]
    fn hex_strings_reject_invalid_input() {
        for hex in [
            "",
            "#",
            "12",
            "12345",
            "1234567",
            "123456789",
            "##123",
            "0x4E85BE",
            "#GG85BE",
            "12345g",
            " 123",
            "123 ",
            "\u{00e9}12",
            "\u{ff14}\u{ff25}",
        ] {
            assert_eq!(Color::from_hex(hex), None, "{hex:?}");
        }
    }
}
