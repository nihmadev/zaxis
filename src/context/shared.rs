//! Resources shared by every window of an application.

use super::Context;
use crate::{
    images::{ImageMetrics, SharedImages, TextureIds},
    text::GlyphStore,
    FontFamily, ImageDecoder, ImageLimits, Style, Theme,
};
use std::{
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

/// Heavy, window-independent state: the font family, the glyph atlas, the image cache and
/// its limits, and the application appearance.
///
/// One `Context` belongs to one native window and owns everything that is per window: input,
/// focus, hover, pointer capture, IME, popups, tooltips, modals, scrolling, drag, viewport and
/// all retained widget state keyed by [`Id`](crate::Id). Contexts built from clones of one
/// `SharedResources` use one glyph atlas and one image cache, so a glyph or an image is
/// rasterized, decoded and uploaded once however many windows show it. The desktop runner
/// creates every window's context this way.
///
/// Cloning is cheap and yields a handle to the same resources.
///
/// ```
/// use zaxis::{Context, SharedResources, Theme};
/// let resources = SharedResources::new();
/// let mut first = Context::with_shared(&resources);
/// let second = Context::with_shared(&resources);
/// // One call re-themes every window; each applies it on its next pass.
/// resources.set_theme(Theme::light());
/// first.run(|_| {});
/// assert_eq!(first.style(), &Theme::light().resolve());
/// # let _ = second;
/// ```
#[derive(Clone)]
pub struct SharedResources {
    inner: Arc<Inner>,
}

struct Inner {
    family: FontFamily,
    /// `None` leaves monospace text to the system's generic monospace font.
    monospace: Option<FontFamily>,
    glyphs: Arc<Mutex<GlyphStore>>,
    images: SharedImages,
    appearance: Mutex<Appearance>,
    appearance_revision: AtomicU64,
}

#[derive(Default)]
struct Appearance {
    theme: Option<Theme>,
    style: Option<Style>,
}

impl Default for SharedResources {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SharedResources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedResources")
            .field("appearance_revision", &self.appearance_revision())
            .finish_non_exhaustive()
    }
}

impl SharedResources {
    /// Resources with the bundled Inter family and no appearance broadcast yet: every
    /// context keeps its own dark default until [`set_theme`](Self::set_theme) is called.
    pub fn new() -> Self {
        Self::with_fonts(FontFamily::inter())
    }

    /// Replace the font family for every window. System fonts still supply scripts the
    /// family lacks, followed by color emoji.
    pub fn with_fonts(family: FontFamily) -> Self {
        Self::with_font_families(family, FontFamily::default_monospace())
    }

    /// Replace both families for every window: the main one and the one that
    /// `TextFamily::Monospace` text, `Typography::code` and `Text::monospace` use.
    ///
    /// `None` for `monospace` uses the system's generic monospace font, which differs
    /// between platforms; the default is the bundled JetBrains Mono (`bundled-monospace`
    /// feature). A system font is never chosen over a configured or bundled family.
    pub fn with_font_families(family: FontFamily, monospace: Option<FontFamily>) -> Self {
        let ids = TextureIds::default();
        Self {
            inner: Arc::new(Inner {
                family,
                monospace,
                glyphs: Arc::new(Mutex::new(GlyphStore::new(ids.clone()))),
                images: SharedImages::new(ids),
                appearance: Mutex::default(),
                appearance_revision: AtomicU64::new(0),
            }),
        }
    }

    /// Install a theme for every window. Each context applies it at the start of its next
    /// pass and is asked to repaint, so the change reaches visible windows in their next
    /// frame and hidden ones when they are shown again. Windows created later start with it.
    /// A theme installed directly with [`Context::set_theme`] is replaced by the next call.
    pub fn set_theme(&self, theme: Theme) {
        self.broadcast(Appearance {
            theme: Some(theme),
            style: None,
        });
    }

    /// Install a resolved style for every window; see [`set_theme`](Self::set_theme).
    pub fn set_style(&self, style: Style) {
        self.broadcast(Appearance {
            theme: None,
            style: Some(style),
        });
    }

    /// The theme last installed with [`set_theme`](Self::set_theme), if any.
    pub fn theme(&self) -> Option<Theme> {
        self.lock_appearance().theme.clone()
    }

    pub fn image_limits(&self) -> ImageLimits {
        self.inner.images.lock().limits.clone()
    }

    /// Replace the image limits of every window. Decoded images are dropped and reloaded.
    pub fn set_image_limits(&self, limits: ImageLimits) {
        let mut images = self.inner.images.lock();
        images.limits = limits;
        images.clear_decoded();
    }

    /// Register a decoder for every window; see [`Context::add_image_decoder`].
    pub fn add_image_decoder(&self, decoder: Arc<dyn ImageDecoder>) {
        self.inner.images.lock().add_decoder(decoder);
    }

    /// Wake callback for finished image jobs; see [`Context::set_image_waker`].
    pub fn set_image_waker(&self, waker: impl Fn() + Send + Sync + 'static) {
        self.inner.images.lock().set_waker(Some(Arc::new(waker)));
    }

    /// Image cache counters summed over every window.
    pub fn image_metrics(&self) -> ImageMetrics {
        self.inner.images.lock().metrics()
    }

    pub fn font_family(&self) -> &FontFamily {
        &self.inner.family
    }

    /// The configured monospace family, `None` for the system's generic monospace font.
    pub fn monospace_family(&self) -> Option<&FontFamily> {
        self.inner.monospace.as_ref()
    }

    /// Whether `other` is a handle to the same resources.
    pub fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn glyphs(&self) -> Arc<Mutex<GlyphStore>> {
        Arc::clone(&self.inner.glyphs)
    }

    pub(crate) fn images(&self) -> &SharedImages {
        &self.inner.images
    }

    pub(crate) fn appearance_revision(&self) -> u64 {
        self.inner.appearance_revision.load(Ordering::Acquire)
    }

    fn lock_appearance(&self) -> std::sync::MutexGuard<'_, Appearance> {
        self.inner.appearance.lock().expect("appearance mutex")
    }

    fn broadcast(&self, appearance: Appearance) {
        *self.lock_appearance() = appearance;
        self.inner.appearance_revision.fetch_add(1, Ordering::Release);
    }
}

impl Context {
    /// A context for one window that uses `resources`: its glyph atlas, image cache, font
    /// family and broadcast appearance. Create one context per native window.
    pub fn with_shared(resources: &SharedResources) -> Self {
        Self::build(resources.clone())
    }

    /// The resources this context shares with the other windows of its application.
    pub fn shared_resources(&self) -> &SharedResources {
        &self.shared
    }

    /// Apply a pending appearance broadcast. Runs at the start of a pass.
    pub(super) fn sync_appearance(&mut self) {
        let revision = self.shared.appearance_revision();
        if revision == self.appearance_seen {
            return;
        }
        self.appearance_seen = revision;
        let (theme, style) = {
            let appearance = self.shared.lock_appearance();
            (appearance.theme.clone(), appearance.style.clone())
        };
        if let Some(theme) = theme {
            self.set_theme(theme);
        } else if let Some(style) = style {
            self.set_style(style);
        }
    }

    /// Whether another window changed what this one shows: a new appearance, or finished
    /// image work for an image it draws. Hidden-from-images windows are not woken.
    pub(super) fn shared_state_changed(&self) -> bool {
        self.shared.appearance_revision() != self.appearance_seen
            || (self.shows_images && {
                let images = self.images.lock();
                images.has_results() || images.epoch() != self.images_epoch
            })
    }
}
