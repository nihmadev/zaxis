use zaxis::Color;

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
