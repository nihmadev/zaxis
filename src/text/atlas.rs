//! Paged glyph atlas. Glyphs are rasterized by swash from the exact face and
//! weight named in the cosmic-text cache key, so weights never share entries.

use super::*;

pub(super) struct AtlasPage {
    pub(super) image: TextureImage,
    x: u32,
    y: u32,
    row_height: u32,
}

impl AtlasPage {
    fn new(id: TextureId, side: u32) -> Self {
        Self {
            image: TextureImage {
                id,
                size: [side; 2],
                pixels: Arc::new([255, 255, 255, 0].repeat((side * side) as usize)),
                revision: 0,
            },
            x: 1,
            y: 1,
            row_height: 0,
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        let side = self.image.size[0];
        if width + 2 > side || height + 2 > side {
            return None;
        }
        if self.x + width + 1 > side {
            self.x = 1;
            self.y += self.row_height + 2;
            self.row_height = 0;
        }
        if self.y + height + 1 > side {
            return None;
        }
        let origin = [self.x, self.y];
        self.x += width + 2;
        self.row_height = self.row_height.max(height);
        Some(origin)
    }
}

impl TextSystem {
    pub(super) fn glyph(&mut self, key: CacheKey) -> Option<CachedGlyph> {
        if let Some(cached) = self.glyphs.get(&key) {
            return *cached;
        }
        let mut fonts = font_system().lock().unwrap();
        let Some(image) = self.swash.get_image_uncached(&mut fonts, key) else {
            self.glyphs.insert(key, None);
            return None;
        };
        drop(fonts);
        let width = image.placement.width;
        let height = image.placement.height;
        if width == 0 || height == 0 {
            self.glyphs.insert(key, None);
            return None;
        }
        let mut allocation = self
            .pages
            .iter_mut()
            .enumerate()
            .find_map(|(i, page)| page.allocate(width, height).map(|origin| (i, origin)));
        if allocation.is_none() {
            let side = ATLAS_SIZE.max((width.max(height) + 2).next_power_of_two());
            let mut page = AtlasPage::new(self.ids.next(), side);
            let origin = page
                .allocate(width, height)
                .expect("a new atlas page fits the glyph");
            allocation = Some((self.pages.len(), origin));
            self.pages.push(page);
        }
        let (page_index, [x, y]) = allocation.unwrap();
        let page = &mut self.pages[page_index];
        let side = page.image.size[0];
        let pixels = Arc::make_mut(&mut page.image.pixels);
        for gy in 0..height {
            for gx in 0..width {
                let source = (gy * width + gx) as usize;
                let rgba = match image.content {
                    SwashContent::Mask => [255, 255, 255, image.data[source]],
                    SwashContent::Color => {
                        let p = &image.data[source * 4..source * 4 + 4];
                        // Swash color images are premultiplied; atlas textures are straight RGBA.
                        let alpha = u32::from(p[3]);
                        let unpremultiply = |c: u8| {
                            (u32::from(c) * 255)
                                .checked_div(alpha)
                                .unwrap_or(0)
                                .min(255) as u8
                        };
                        [
                            unpremultiply(p[0]),
                            unpremultiply(p[1]),
                            unpremultiply(p[2]),
                            p[3],
                        ]
                    }
                    SwashContent::SubpixelMask => {
                        let p = &image.data[source * 4..source * 4 + 4];
                        [255, 255, 255, p[0].max(p[1]).max(p[2])]
                    }
                };
                let index = (((y + gy) * side + x + gx) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&rgba);
            }
        }
        page.image.revision += 1;
        let glyph = CachedGlyph {
            texture: page.image.id,
            uv: Rect::from_min_size(
                Vec2::new(x as f32, y as f32) / side as f32,
                Vec2::new(width as f32, height as f32) / side as f32,
            ),
            offset: Vec2::new(image.placement.left as f32, -image.placement.top as f32),
            colored: image.content == SwashContent::Color,
            size: Vec2::new(width as f32, height as f32),
        };
        self.glyphs.insert(key, Some(glyph));
        Some(glyph)
    }

    pub fn textures(&self) -> Vec<TextureImage> {
        self.pages.iter().map(|page| page.image.clone()).collect()
    }
}
