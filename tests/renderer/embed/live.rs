//! A host in miniature for images that change in place: a context drawing one image into a
//! target, an embedded renderer presenting it and a model of the pixels it must show.

use super::{
    harness::{host, Host, Spec},
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{
    vec2, Color, Context, EmbedViewport, EmbeddedRenderer, Image, ImageFit, ImageHandle,
    ImageSource, RendererStats, Root, TextureFilter, TextureRegion,
};

/// How the image is drawn: by default one image pixel to one target pixel.
#[derive(Clone, Copy)]
pub struct View {
    pub target: [u32; 2],
    pub tint: Color,
    pub opacity: f32,
    pub radius: f32,
    pub fit: ImageFit,
    pub filter: TextureFilter,
    /// A backdrop blur over this rectangle (logical pixels) and radius, drawn after the image.
    pub blur: Option<(zaxis::Rect, f32)>,
}

impl View {
    pub fn exact(size: [u32; 2]) -> Self {
        Self {
            blur: None,
            target: size,
            tint: Color::WHITE,
            opacity: 1.0,
            radius: 0.0,
            fit: ImageFit::Stretch,
            filter: TextureFilter::Linear,
        }
    }
}

pub struct Rig<'h> {
    pub host: &'h Host,
    pub spec: Spec,
    pub embed: EmbeddedRenderer,
    pub ctx: Context,
    pub handle: ImageHandle,
    pub view: View,
}

pub fn embedded(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .unwrap()
}

impl<'h> Rig<'h> {
    pub fn new(host: &'h Host, source: ImageSource, view: View) -> Self {
        let spec = Spec::new(F::Rgba8UnormSrgb, view.target);
        let mut ctx = scene::context(view.target, 1.0);
        let handle = ctx.load_image(source).unwrap();
        Self {
            embed: embedded(host, &spec),
            host,
            spec,
            ctx,
            handle,
            view,
        }
    }

    /// One pass of the context: the image over the whole target.
    pub fn build(&mut self) {
        let (handle, view) = (self.handle, self.view);
        self.ctx.run(|ctx| {
            Root::new().padding(Default::default()).show(ctx, |ui| {
                let rect = zaxis::Rect::from_min_size(
                    vec2(0.0, 0.0),
                    vec2(view.target[0] as f32, view.target[1] as f32),
                );
                ui.at("image", rect, |ui| {
                    ui.add(
                        Image::new(handle)
                            .size(rect.size())
                            .fit(view.fit)
                            .tint(view.tint)
                            .opacity(view.opacity)
                            .corner_radius(view.radius)
                            .filter(view.filter),
                    );
                    if let Some((rect, radius)) = view.blur {
                        zaxis::Blur::new(rect).radius(radius).show(ui);
                    }
                });
            });
        });
    }

    /// Frames until the image is ready and no decode is in flight.
    pub fn settle(&mut self) {
        let start = std::time::Instant::now();
        loop {
            self.build();
            let ready = self.ctx.image_state(self.handle).is_ready();
            if ready && self.ctx.image_metrics().pending_jobs == 0 && !self.ctx.needs_repaint() {
                return;
            }
            assert!(start.elapsed().as_secs() < 10, "image never became ready");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    /// Run the context, present the frame into a host pass and read it back.
    pub fn pass(&mut self) -> Vec<[u8; 4]> {
        self.build();
        self.embed
            .prepare(self.ctx.draw_data(), EmbedViewport::whole(self.view.target))
            .unwrap();
        let embed = &self.embed;
        self.host.in_pass(&self.spec, false, |pass| {
            embed.record(pass).unwrap();
        })
    }

    /// The same through `render_to` into a texture of the host.
    pub fn texture(&mut self) -> Vec<[u8; 4]> {
        self.build();
        let data = self.ctx.draw_data();
        let embed = &mut self.embed;
        self.host.frame(
            &self.spec,
            false,
            wgpu::TextureUsages::empty(),
            |encoder, target| {
                embed
                    .render_to(
                        encoder,
                        &target.view,
                        None,
                        zaxis::EmbedLoad::Clear(Color::BLACK),
                        data,
                    )
                    .unwrap();
            },
        )
    }

    pub fn stats(&self) -> RendererStats {
        self.embed.stats()
    }
}

/// A pass of a new context and renderer showing `source` in the same view: what the
/// pixels must be when nothing was ever updated.
pub fn fresh(host: &Host, source: ImageSource, view: View) -> Vec<[u8; 4]> {
    let mut rig = Rig::new(host, source, view);
    rig.settle();
    rig.pass()
}

/// A deterministic stream of bytes.
pub struct Noise(pub u64);

impl Noise {
    pub fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    pub fn bytes(&mut self, n: usize, alpha: Option<u8>) -> Vec<u8> {
        let mut out: Vec<u8> = (0..n).map(|_| self.next() as u8).collect();
        if let Some(a) = alpha {
            out.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = a);
        }
        out
    }
}

/// `host`, or a message and `None`.
pub fn gpu(name: &str) -> Option<Host> {
    host(name)
}

/// The model as target-sized sRGB pixels.
pub fn expected(model: &[u8]) -> Vec<[u8; 4]> {
    model.as_chunks::<4>().0.to_vec()
}

pub fn region_rows(
    model: &mut [u8],
    width: u32,
    region: TextureRegion,
    noise: &mut Noise,
    alpha: Option<u8>,
    pad: usize,
) -> (Vec<u8>, usize) {
    let row = region.width as usize * 4;
    let stride = row + pad;
    let mut host = noise.bytes(stride * region.height as usize, alpha);
    for y in 0..region.height as usize {
        let to = ((region.y as usize + y) * width as usize + region.x as usize) * 4;
        model[to..to + row].copy_from_slice(&host[y * stride..][..row]);
        // What lies between rows belongs to nobody; make sure nothing reads it.
        host[y * stride + row..(y + 1) * stride].fill(0xA5);
    }
    (host, stride)
}
