//! A stand-in for a page renderer: a BGRA frame where a box moves and a ticker advances, and
//! the rectangles each step changed.
use zaxis::TextureRegion;

pub const SIZE: [u32; 2] = [640, 400];
const BOX: [u32; 2] = [96, 64];
const TICKER: u32 = 18;

pub struct Page {
    pub pixels: Vec<u8>,
    step: u32,
    place: [u32; 2],
}

fn background(x: u32, y: u32) -> [u8; 4] {
    [
        (x * 255 / SIZE[0]) as u8,
        (y * 255 / SIZE[1]) as u8,
        96,
        255,
    ]
}

impl Page {
    pub fn new() -> Self {
        let mut pixels = Vec::with_capacity((SIZE[0] * SIZE[1] * 4) as usize);
        for y in 0..SIZE[1] {
            for x in 0..SIZE[0] {
                pixels.extend(background(x, y));
            }
        }
        let mut page = Self {
            pixels,
            step: 0,
            place: [0, 0],
        };
        page.fill(
            TextureRegion::new(0, 0, BOX[0], BOX[1]),
            [40, 200, 255, 255],
        );
        page
    }

    fn fill(&mut self, region: TextureRegion, color: [u8; 4]) {
        for y in region.y..region.y + region.height {
            for x in region.x..region.x + region.width {
                let at = ((y * SIZE[0] + x) * 4) as usize;
                self.pixels[at..at + 4].copy_from_slice(&color);
            }
        }
    }

    fn restore(&mut self, region: TextureRegion) {
        for y in region.y..region.y + region.height {
            for x in region.x..region.x + region.width {
                let at = ((y * SIZE[0] + x) * 4) as usize;
                self.pixels[at..at + 4].copy_from_slice(&background(x, y));
            }
        }
    }

    /// Advance one step; the rectangles of the frame that changed.
    pub fn step(&mut self) -> Vec<TextureRegion> {
        self.step += 1;
        let old = TextureRegion::new(self.place[0], self.place[1], BOX[0], BOX[1]);
        let travel = [SIZE[0] - BOX[0], SIZE[1] - TICKER - BOX[1]];
        let phase = |n: u32, span: u32| {
            let t = n % (2 * span);
            if t < span {
                t
            } else {
                2 * span - t
            }
        };
        self.place = [
            phase(self.step * 3, travel[0]),
            phase(self.step * 2, travel[1]),
        ];
        let new = TextureRegion::new(self.place[0], self.place[1], BOX[0], BOX[1]);
        self.restore(old);
        self.fill(new, [40, 200, 255, 255]);
        let ticker = TextureRegion::new(0, SIZE[1] - TICKER, SIZE[0], TICKER);
        self.restore(ticker);
        let bar = TextureRegion::new(0, SIZE[1] - TICKER + 4, (self.step * 5) % SIZE[0], 10);
        if !bar.is_empty() {
            self.fill(bar, [255, 255, 255, 255]);
        }
        vec![old.union(new), ticker]
    }

    /// The pixels from the first row of `region`, for `update_image_region`, and their stride.
    pub fn rows(&self, region: TextureRegion) -> (&[u8], usize) {
        let first = ((region.y * SIZE[0] + region.x) * 4) as usize;
        (&self.pixels[first..], (SIZE[0] * 4) as usize)
    }
}
