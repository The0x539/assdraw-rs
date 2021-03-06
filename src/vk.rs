use std::sync::Arc;

use native_windows_gui as nwg;

use vulkano::{
    device::{Device, DeviceExtensions, Features, Queue},
    image::{swapchain::SwapchainImage, ImageUsage},
    instance::{Instance, InstanceExtensions, PhysicalDevice},
    swapchain::{CompositeAlpha, FullscreenExclusive, PresentMode, Surface, Swapchain},
    sync::SharingMode,
};

fn make_extern_canvas<W: Into<nwg::ControlHandle>>(parent: W) -> nwg::ExternCanvas {
    let mut c = nwg::ExternCanvas::default();
    nwg::ExternCanvas::builder()
        .parent(Some(parent.into()))
        .build(&mut c)
        .expect("Failed to build nwg::ExternCanvas");
    c
}

#[allow(dead_code)]
pub struct VkCanvas {
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<()>>,
    swapchain: Arc<Swapchain<()>>,
    buffers: Vec<Arc<SwapchainImage<()>>>,

    // drop order is important
    canvas: nwg::ExternCanvas,
}

#[allow(dead_code)]
impl VkCanvas {
    pub fn new<W: Into<nwg::ControlHandle>>(parent: W) -> Self {
        use std::ptr;
        use winapi::shared::minwindef::HINSTANCE;

        let canvas = make_extern_canvas(parent);

        let extensions = InstanceExtensions {
            khr_surface: true,
            khr_win32_surface: true,
            ..InstanceExtensions::none()
        };
        let instance = Instance::new(None, &extensions, None).unwrap();

        let mut device_and_queue = None;
        let features = Features { ..Features::none() };
        let dev_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };
        for gpu in PhysicalDevice::enumerate(&instance) {
            let family = match gpu.queue_families().find(|q| q.supports_graphics()) {
                Some(fam) => fam,
                None => continue,
            };

            let fams = std::iter::once((family, 0.5));
            let (device, mut queues) = Device::new(gpu, &features, &dev_extensions, fams).unwrap();
            device_and_queue = Some((device, queues.next().expect("no queues")));
            break;
        }
        let (device, queue) = device_and_queue.expect("no device");

        let hinstance: HINSTANCE = ptr::null_mut();
        let hwnd = canvas.handle.hwnd().expect("Canvas was uninitialized");
        let surface = unsafe { Surface::from_hwnd(instance.clone(), hinstance, hwnd, ()).unwrap() };

        let (swapchain, buffers) = {
            let caps = surface.capabilities(device.physical_device()).unwrap();
            let dimensions = caps.current_extent.unwrap_or([100, 100]);
            let n_buffers = 2
                .max(caps.min_image_count)
                .min(caps.max_image_count.unwrap_or(u32::MAX));
            let transform = caps.current_transform;
            let (format, color_space) = caps.supported_formats[0];
            let usage = ImageUsage {
                color_attachment: true,
                ..ImageUsage::none()
            };

            Swapchain::new(
                device.clone(),
                surface.clone(),
                n_buffers,
                format,
                dimensions,
                1,
                usage,
                SharingMode::Exclusive,
                transform, // SurfaceTransform::Inherit,
                CompositeAlpha::Opaque,
                PresentMode::Relaxed,
                FullscreenExclusive::Default,
                true,
                color_space, //ColorSpace::SrgbNonLinear,
            )
            .unwrap()
        };

        Self {
            instance,
            device,
            queue,
            surface,
            swapchain,
            buffers,

            canvas,
        }
    }

    pub fn handle(&self) -> &nwg::ControlHandle {
        &self.canvas.handle
    }

    pub fn resize(&self) {}
    pub fn set_image<T>(&self, _: T) {}
    pub fn update_dimensions<T: FnOnce(&mut crate::gl::Dimensions)>(&self, _: T) {}
    pub fn add_point<T, U>(&self, _: T, _: U) {}
    pub fn get_dimensions(&self) -> crate::gl::Dimensions {
        todo!()
    }
    pub fn pop_point(&self) {}
    pub fn render(&self) {}
}
