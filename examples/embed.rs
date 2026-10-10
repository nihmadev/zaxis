//! cargo run --example embed [-- --pass]
//! A host with its own window, wgpu device and 3D scene; zaxis draws tools over it without a
//! window or surface of its own. By default the interface is rendered into the host's texture
//! after the scene, so the glass panels blur the cube; `--pass` records it into the scene's
//! MSAA and depth pass instead, where the glass degrades to what a pass can draw.
//! `--smoke-test` renders both modes off screen and checks the pixels.

#[path = "embed/frame.rs"]
mod frame;
#[path = "embed/gpu.rs"]
mod gpu;
#[path = "embed/interface.rs"]
mod interface;
#[path = "embed/model.rs"]
mod model;
#[path = "embed/scene.rs"]
mod scene;
#[path = "embed/smoke.rs"]
mod smoke;
#[path = "embed/targets.rs"]
mod targets;
#[path = "embed/window.rs"]
mod window;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    if arguments.any(|a| a == "--smoke-test") {
        return smoke::run();
    }
    window::run(std::env::args().any(|a| a == "--pass"))
}
