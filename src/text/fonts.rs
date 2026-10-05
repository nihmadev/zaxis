//! The process-wide font database: registered families, system fonts for
//! scripts, and the bundled color emoji, in that fallback order.

use std::sync::{Arc, Mutex, OnceLock};

use cosmic_text::{fontdb, Fallback, FontSystem, PlatformFallback};

use super::{family::FontFamily, weight::FontWeight};
use crate::Id;

/// The shaped-text fallback chain. cosmic-text tries the requested family
/// first, then the platform's per-script families, then the entries below, and
/// finally any installed face. Emoji sort ahead of the platform's common list.
struct EmojiFallback(Vec<&'static str>);

impl Fallback for EmojiFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &self.0
    }
    fn forbidden_fallback(&self) -> &[&'static str] {
        PlatformFallback.forbidden_fallback()
    }
    fn script_fallback(&self, script: unicode_script::Script, locale: &str) -> &[&'static str] {
        PlatformFallback.script_fallback(script, locale)
    }
}

/// cosmic-text only accepts a fallback face whose weight equals the one being
/// shaped. The single-weight emoji font is therefore registered once per
/// weight step, as aliases of the same data, so it follows every request.
#[cfg(feature = "bundled-emoji")]
const ALIAS_WEIGHTS: [FontWeight; 9] = [
    FontWeight::THIN,
    FontWeight::EXTRA_LIGHT,
    FontWeight::LIGHT,
    FontWeight::REGULAR,
    FontWeight::MEDIUM,
    FontWeight::SEMIBOLD,
    FontWeight::BOLD,
    FontWeight::EXTRA_BOLD,
    FontWeight::BLACK,
];

pub fn font_system() -> &'static Mutex<FontSystem> {
    static FONTS: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    FONTS.get_or_init(|| {
        let system = FontSystem::new();
        let (locale, mut db) = system.into_locale_and_db();
        #[cfg(feature = "bundled-emoji")]
        // Static data stays mapped from the executable instead of a heap copy.
        add_faces(
            &mut db,
            "Noto Color Emoji",
            Arc::new(z_emoji::FONT_DATA),
            &ALIAS_WEIGHTS,
        );
        let common = std::iter::once("Noto Color Emoji")
            .chain(PlatformFallback.common_fallback().iter().copied())
            .collect();
        Mutex::new(FontSystem::new_with_locale_and_db_and_fallback(
            locale,
            db,
            EmojiFallback(common),
        ))
    })
}

/// Load `data` once per listed weight under one family name. Fonts are parsed
/// by cosmic-text only when a face is first shaped, so unused weights cost one
/// index entry.
fn add_faces(
    db: &mut fontdb::Database,
    name: &str,
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    weights: &[FontWeight],
) {
    let mut parsed = fontdb::Database::new();
    parsed.load_font_source(fontdb::Source::Binary(data));
    for face in parsed.faces() {
        for weight in weights {
            let mut face = face.clone();
            face.families = vec![(name.to_owned(), face.families[0].1)];
            face.weight = weight.to_fontdb();
            db.push_face_info(face);
        }
    }
}

/// A family registered in the shared database.
#[derive(Clone, Debug)]
pub(super) struct Registered {
    pub name: String,
    pub id: Id,
    pub family: FontFamily,
}

/// Register the files of `family` under a name unique to their content, so a
/// custom font never collides with an installed font of the same family name.
/// Registering the same fonts again, for example for a second `Context`, is free.
pub(super) fn register(family: FontFamily) -> Registered {
    let id = family.identity();
    let name = format!("zaxis-{id:?}");
    let mut fonts = font_system().lock().unwrap();
    let known = fonts
        .db()
        .faces()
        .any(|face| face.families.iter().any(|(n, _)| n == &name));
    if !known {
        let db = fonts.db_mut();
        for (weight, data) in family.faces() {
            add_faces(db, &name, Arc::clone(data), &[weight]);
        }
    }
    Registered { name, id, family }
}
