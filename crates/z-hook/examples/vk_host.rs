//! A minimal Vulkan application to run the layer against, with no window: it renders to a
//! headless swapchain, clearing every frame to one color, so whatever else is in the frame
//! is the overlay.
//!
//! `vk_host [--frames N] [--format unorm|srgb] [--size WxH] [--resize WxH] [--api 1.0|1.3] [--delay MS]`
//!
//! `--resize` recreates the swapchain half-way through, to exercise the layer's handling of
//! swapchain recreation. The final line says how many frames were presented.

use ash::{khr, vk, Entry};
use std::process::ExitCode;

struct Args {
    frames: u32,
    format: vk::Format,
    size: (u32, u32),
    resize: Option<(u32, u32)>,
    api: u32,
    /// Milliseconds to wait after each frame, so a short run outlasts the overlay's start-up.
    delay: u64,
}

fn parse() -> Args {
    let mut args = Args {
        frames: 120,
        format: vk::Format::B8G8R8A8_UNORM,
        size: (640, 480),
        resize: None,
        api: vk::API_VERSION_1_3,
        delay: 0,
    };
    let mut it = std::env::args().skip(1);
    let size = |s: &str| {
        let (w, h) = s.split_once('x').expect("WxH");
        (w.parse().expect("width"), h.parse().expect("height"))
    };
    while let Some(arg) = it.next() {
        let value = it.next().expect("a value after each flag");
        match arg.as_str() {
            "--frames" => args.frames = value.parse().expect("frame count"),
            "--format" => {
                args.format = if value == "srgb" {
                    vk::Format::B8G8R8A8_SRGB
                } else {
                    vk::Format::B8G8R8A8_UNORM
                }
            }
            "--size" => args.size = size(&value),
            "--resize" => args.resize = Some(size(&value)),
            "--delay" => args.delay = value.parse().expect("milliseconds"),
            "--api" => {
                args.api = if value == "1.0" {
                    vk::API_VERSION_1_0
                } else {
                    vk::API_VERSION_1_3
                }
            }
            other => panic!("unknown flag {other}"),
        }
    }
    args
}

/// Every object the swapchain owns, to rebuild on resize.
struct Frames {
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    /// One per image: a present's wait semaphore is free again only once its image is re-acquired.
    rendered: Vec<vk::Semaphore>,
}

fn main() -> ExitCode {
    match run(parse()) {
        Ok(presented) => {
            println!("vk_host: presented {presented} frames");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("vk_host: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<u32, String> {
    // SAFETY: plain Vulkan use; every object is destroyed before its parent, after the device is idle.
    unsafe {
        let entry = Entry::load().map_err(|e| e.to_string())?;
        let app = vk::ApplicationInfo::default()
            .api_version(args.api)
            .application_name(c"vk_host");
        let extensions = [
            khr::surface::NAME.as_ptr(),
            ash::ext::headless_surface::NAME.as_ptr(),
        ];
        let instance = entry
            .create_instance(
                &vk::InstanceCreateInfo::default()
                    .application_info(&app)
                    .enabled_extension_names(&extensions),
                None,
            )
            .map_err(|e| format!("instance: {e}"))?;
        let surface_fn = khr::surface::Instance::new(&entry, &instance);
        let headless = ash::ext::headless_surface::Instance::new(&entry, &instance);
        let surface = headless
            .create_headless_surface(&vk::HeadlessSurfaceCreateInfoEXT::default(), None)
            .map_err(|e| format!("surface: {e}"))?;
        let physical = instance
            .enumerate_physical_devices()
            .map_err(|e| e.to_string())?[0];
        let family = instance
            .get_physical_device_queue_family_properties(physical)
            .iter()
            .position(|f| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or("no graphics queue")? as u32;
        let priorities = [1.0];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(family)
            .queue_priorities(&priorities)];
        let device_extensions = [khr::swapchain::NAME.as_ptr()];
        let device = instance
            .create_device(
                physical,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queues)
                    .enabled_extension_names(&device_extensions),
                None,
            )
            .map_err(|e| format!("device: {e}"))?;
        let swapchain_fn = khr::swapchain::Device::new(&instance, &device);
        let queue = device.get_device_queue(family, 0);
        let pool = device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(family)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
            .map_err(|e| e.to_string())?;
        let commands = device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .command_buffer_count(1),
            )
            .map_err(|e| e.to_string())?;
        let cmd = commands[0];
        let acquired = device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .map_err(|e| e.to_string())?;
        let fence = device
            .create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
            .map_err(|e| e.to_string())?;

        let make = |size: (u32, u32), old: vk::SwapchainKHR| -> Result<Frames, String> {
            let caps = surface_fn
                .get_physical_device_surface_capabilities(physical, surface)
                .map_err(|e| e.to_string())?;
            let extent = vk::Extent2D {
                width: size.0,
                height: size.1,
            };
            let info =
                vk::SwapchainCreateInfoKHR::default()
                    .surface(surface)
                    .min_image_count(caps.min_image_count.max(3).min(
                        if caps.max_image_count == 0 {
                            u32::MAX
                        } else {
                            caps.max_image_count
                        },
                    ))
                    .image_format(args.format)
                    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(
                        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                    )
                    .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .clipped(true)
                    .old_swapchain(old);
            let swapchain = swapchain_fn
                .create_swapchain(&info, None)
                .map_err(|e| format!("swapchain: {e}"))?;
            let images = swapchain_fn
                .get_swapchain_images(swapchain)
                .map_err(|e| e.to_string())?;
            let rendered = images
                .iter()
                .map(|_| {
                    device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Frames {
                swapchain,
                images,
                rendered,
            })
        };
        let mut frames = make(args.size, vk::SwapchainKHR::null())?;
        let mut presented = 0;
        let mut times = Vec::with_capacity(args.frames as usize);
        for frame in 0..args.frames {
            let started = std::time::Instant::now();
            if let (Some(size), true) = (args.resize, frame == args.frames / 2) {
                device.device_wait_idle().map_err(|e| e.to_string())?;
                let next = make(size, frames.swapchain)?;
                swapchain_fn.destroy_swapchain(frames.swapchain, None);
                frames
                    .rendered
                    .iter()
                    .for_each(|s| device.destroy_semaphore(*s, None));
                frames = next;
            }
            device
                .wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| e.to_string())?;
            device.reset_fences(&[fence]).map_err(|e| e.to_string())?;
            let (index, _) = swapchain_fn
                .acquire_next_image(frames.swapchain, u64::MAX, acquired, vk::Fence::null())
                .map_err(|e| format!("acquire: {e}"))?;
            let image = frames.images[index as usize];
            device
                .begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .map_err(|e| e.to_string())?;
            let range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);
            let barrier = |old, new, src, dst| {
                vk::ImageMemoryBarrier::default()
                    .old_layout(old)
                    .new_layout(new)
                    .src_access_mask(src)
                    .dst_access_mask(dst)
                    .image(image)
                    .subresource_range(range)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            };
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier(
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::TRANSFER_WRITE,
                )],
            );
            device.cmd_clear_color_image(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearColorValue {
                    float32: [0.2, 0.35, 0.2, 1.0],
                },
                &[range],
            );
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier(
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    vk::AccessFlags::TRANSFER_WRITE,
                    vk::AccessFlags::empty(),
                )],
            );
            device.end_command_buffer(cmd).map_err(|e| e.to_string())?;
            let wait = [acquired];
            let stages = [vk::PipelineStageFlags::TRANSFER];
            let signal = [frames.rendered[index as usize]];
            let buffers = [cmd];
            device
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default()
                        .wait_semaphores(&wait)
                        .wait_dst_stage_mask(&stages)
                        .command_buffers(&buffers)
                        .signal_semaphores(&signal)],
                    fence,
                )
                .map_err(|e| e.to_string())?;
            let swapchains = [frames.swapchain];
            let indices = [index];
            swapchain_fn
                .queue_present(
                    queue,
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(&signal)
                        .swapchains(&swapchains)
                        .image_indices(&indices),
                )
                .map_err(|e| format!("present: {e}"))?;
            presented += 1;
            times.push(started.elapsed());
            if args.delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(args.delay));
            }
        }
        device.device_wait_idle().map_err(|e| e.to_string())?;
        // The last third of the run: after any start-up of the layer, with the overlay at rest.
        let from = times.len() * 2 / 3;
        let tail = &mut times[from..];
        tail.sort();
        if !tail.is_empty() {
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
            println!(
                "vk_host: frame ms p50 {:.3} p95 {:.3} max {:.3}",
                ms(tail[tail.len() / 2]),
                ms(tail[tail.len() * 95 / 100]),
                ms(tail[tail.len() - 1])
            );
        }
        swapchain_fn.destroy_swapchain(frames.swapchain, None);
        frames
            .rendered
            .iter()
            .for_each(|s| device.destroy_semaphore(*s, None));
        device.destroy_fence(fence, None);
        device.destroy_semaphore(acquired, None);
        device.destroy_command_pool(pool, None);
        device.destroy_device(None);
        surface_fn.destroy_surface(surface, None);
        instance.destroy_instance(None);
        Ok(presented)
    }
}
