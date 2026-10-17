//! GPU renderer foundation for PLITA UI.
//!
//! Phase 1 keeps the Phase 0 winit/wgpu/Vello foundation and adds a small GPU
//! rectangle renderer for the first creative-app UI shell.

use std::{collections::HashMap, sync::Arc, time::Instant};

use bytemuck::{Pod, Zeroable};
use plita_ui::{Color, ColoredVertex, DrawCommand, DrawList, Rect, Size};
use thiserror::Error;
use tracing::{debug, info, warn};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

/// RGBA clear color used by the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearColor {
    /// Red channel in linear floating point range.
    pub r: f64,
    /// Green channel in linear floating point range.
    pub g: f64,
    /// Blue channel in linear floating point range.
    pub b: f64,
    /// Alpha channel in linear floating point range.
    pub a: f64,
}

impl ClearColor {
    /// Create an opaque RGB color.
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create an RGBA color.
    pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

impl From<Color> for ClearColor {
    fn from(value: Color) -> Self {
        Self {
            r: f64::from(value.r),
            g: f64::from(value.g),
            b: f64::from(value.b),
            a: f64::from(value.a),
        }
    }
}

/// Renderer configuration passed from the window/application layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderConfig {
    /// Background color for the first frame.
    pub clear_color: ClearColor,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            clear_color: ClearColor::rgb(0.105, 0.111, 0.125),
        }
    }
}

/// Physical surface size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl RenderSize {
    /// Create a size, clamping both dimensions to at least 1.
    pub const fn clamped(width: u32, height: u32) -> Self {
        Self {
            width: if width == 0 { 1 } else { width },
            height: if height == 0 { 1 } else { height },
        }
    }

    /// Convert to the UI `Size` type.
    pub fn to_ui_size(self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }

    /// Convert the physical surface size to DPI-independent logical UI units.
    pub fn to_ui_size_with_scale_factor(self, scale_factor: f64) -> Size {
        let scale_factor = normalized_scale_factor(scale_factor);
        Size::new(
            (f64::from(self.width) / scale_factor) as f32,
            (f64::from(self.height) / scale_factor) as f32,
        )
    }
}

fn normalized_scale_factor(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

impl From<PhysicalSize<u32>> for RenderSize {
    fn from(value: PhysicalSize<u32>) -> Self {
        Self::clamped(value.width, value.height)
    }
}

/// Frame render statistics useful for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStats {
    /// Presented frame index.
    pub frame_index: u64,
    /// Surface width in physical pixels.
    pub width: u32,
    /// Surface height in physical pixels.
    pub height: u32,
    /// Number of UI draw commands submitted to the rectangle renderer.
    pub draw_commands: usize,
    /// CPU-side wall time spent building/submitting this frame.
    pub cpu_frame_ms: f64,
}

/// Errors from the renderer layer.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Surface creation failed.
    #[error("failed to create WGPU surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    /// Adapter request failed.
    #[error("failed to request a compatible GPU adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    /// Device request failed.
    #[error("failed to request WGPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    /// Surface and adapter are not compatible.
    #[error("surface is not compatible with the selected adapter")]
    SurfaceUnsupported,

    /// Vello renderer setup failed.
    #[error("failed to initialize Vello renderer: {0}")]
    Vello(String),

    /// Surface presentation error.
    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    /// An opted-in presentation worker exited unexpectedly.
    #[error("GPU presentation worker disconnected")]
    WorkerDisconnected,
}

impl RenderError {
    /// True when frame acquisition timed out and a later redraw can retry.
    ///
    /// This helper keeps higher layers from depending directly on `wgpu`.
    pub fn is_surface_timeout(&self) -> bool {
        matches!(self, Self::Surface(wgpu::SurfaceError::Timeout))
    }

    /// True when the GPU reports an out-of-memory condition.
    ///
    /// This is treated as fatal by the window loop.
    pub fn is_surface_out_of_memory(&self) -> bool {
        matches!(self, Self::Surface(wgpu::SurfaceError::OutOfMemory))
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct UiVertex {
    position: [f32; 2],
    color: [f32; 4],
}

const UI_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

impl UiVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &UI_VERTEX_ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ImageVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

const IMAGE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

impl ImageVertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &IMAGE_VERTEX_ATTRIBUTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ImageCacheKey {
    slot: usize,
    shared: bool,
    len: usize,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImageVertexCacheKey {
    rect: [u32; 4],
    surface_width: u32,
    surface_height: u32,
}

// Retaining the source prevents allocator address reuse from aliasing cached
// textures. Arc::make_mut also detaches edits from an immutable cached image.
enum ImageSource {
    Immutable(Arc<[u8]>),
    Shared(Arc<std::sync::Mutex<Vec<u8>>>),
}

impl ImageSource {
    fn identity(&self) -> (usize, bool) {
        match self {
            Self::Immutable(pixels) => (pixels.as_ptr() as usize, false),
            Self::Shared(pixels) => (Arc::as_ptr(pixels) as usize, true),
        }
    }
}

struct CachedImage {
    _source: ImageSource,
    last_used_frame: u64,
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    vertex_key: Option<ImageVertexCacheKey>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
}

/// Window-bound surface prepared on the thread that created the window.
///
/// Owns the window through WGPU's safe surface target. Device and pipeline
/// initialization may consume this on a worker without querying `Window`.
pub struct PreparedSurface {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    physical_size: RenderSize,
    ui_scale_factor: f64,
}

/// GPU renderer.
pub struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    clear_color: wgpu::Color,
    draw_list: DrawList,
    ui_renderer: UiRenderer,
    ui_scale_factor: f64,
    /// Created to prove that the Vello layer can be initialized.
    /// Later phases will feed scenes into this renderer.
    _vello_renderer: vello::Renderer,
    frame_index: u64,
}

impl GpuRenderer {
    /// Create the GPU renderer. Poll this future on the window's owning thread.
    pub async fn new(window: Arc<Window>, config: RenderConfig) -> Result<Self, RenderError> {
        Self::from_prepared_surface(Self::prepare_surface(window)?, config).await
    }

    /// Capture window properties and create its surface on the owning thread.
    pub fn prepare_surface(window: Arc<Window>) -> Result<PreparedSurface, RenderError> {
        let physical_size = RenderSize::from(window.inner_size());
        let ui_scale_factor = normalized_scale_factor(window.scale_factor());

        let instance_descriptor = plita_instance_descriptor();
        info!(
            requested_backends = ?instance_descriptor.backends,
            compiled_backends = ?wgpu::Instance::enabled_backend_features(),
            "creating WGPU instance"
        );
        let instance = wgpu::Instance::new(&instance_descriptor);
        let surface = instance.create_surface(window)?;
        Ok(PreparedSurface {
            instance,
            surface,
            physical_size,
            ui_scale_factor,
        })
    }

    /// Initialize GPU resources without accessing the native window handle.
    pub async fn from_prepared_surface(
        prepared: PreparedSurface,
        config: RenderConfig,
    ) -> Result<Self, RenderError> {
        let PreparedSurface {
            instance,
            surface,
            physical_size,
            ui_scale_factor,
        } = prepared;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let adapter_info = adapter.get_info();
        info!(
            name = %adapter_info.name,
            backend = ?adapter_info.backend,
            device_type = ?adapter_info.device_type,
            "selected GPU adapter"
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("plita.phase1.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        let mut surface_config = surface
            .get_default_config(&adapter, physical_size.width, physical_size.height)
            .ok_or(RenderError::SurfaceUnsupported)?;

        // GUI work should favor predictable latency. Fifo is universally supported.
        surface_config.present_mode = wgpu::PresentMode::Fifo;
        surface_config.desired_maximum_frame_latency = 1;
        surface.configure(&device, &surface_config);

        let vello_renderer = vello::Renderer::new(&device, vello::RendererOptions::default())
            .map_err(|err| RenderError::Vello(err.to_string()))?;

        let ui_renderer = UiRenderer::new(&device, surface_config.format);

        info!(
            width = surface_config.width,
            height = surface_config.height,
            format = ?surface_config.format,
            present_mode = ?surface_config.present_mode,
            "GPU renderer initialized"
        );

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            surface_config,
            clear_color: config.clear_color.to_wgpu(),
            draw_list: DrawList::new(Color::rgba(
                config.clear_color.r as f32,
                config.clear_color.g as f32,
                config.clear_color.b as f32,
                config.clear_color.a as f32,
            )),
            ui_renderer,
            ui_scale_factor,
            _vello_renderer: vello_renderer,
            frame_index: 0,
        })
    }

    /// Replace the UI draw list used on the next frame.
    pub fn set_draw_list(&mut self, draw_list: DrawList) {
        self.clear_color = ClearColor::from(draw_list.clear_color).to_wgpu();
        self.draw_list = draw_list;
    }

    /// Resize the surface. Zero-sized minimized windows are clamped.
    pub fn resize(&mut self, size: RenderSize) {
        if size.width == self.surface_config.width && size.height == self.surface_config.height {
            return;
        }

        debug!(width = size.width, height = size.height, "resizing surface");
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Current physical surface size.
    pub fn size(&self) -> RenderSize {
        RenderSize {
            width: self.surface_config.width,
            height: self.surface_config.height,
        }
    }

    /// Current surface size in DPI-independent logical UI units.
    pub fn ui_size(&self) -> Size {
        self.size()
            .to_ui_size_with_scale_factor(self.ui_scale_factor)
    }

    /// Update the operating-system DPI scale used to map logical UI coordinates
    /// onto the physical render surface.
    pub fn set_ui_scale_factor(&mut self, scale_factor: f64) {
        self.ui_scale_factor = normalized_scale_factor(scale_factor);
    }

    /// Format selected for the surface.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// True when the selected adapter still reports support for this surface.
    pub fn is_surface_supported(&self) -> bool {
        self.adapter.is_surface_supported(&self.surface)
    }

    /// Render one frame by clearing the surface and drawing the UI commands.
    pub fn render(&mut self) -> Result<FrameStats, RenderError> {
        let start = Instant::now();
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                warn!("surface lost/outdated; reconfiguring");
                self.surface.configure(&self.device, &self.surface_config);
                return self.render();
            }
            Err(err) => return Err(RenderError::Surface(err)),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("plita.phase1.command_encoder"),
            });

        self.ui_renderer.encode(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            self.size(),
            self.ui_size(),
            &self.draw_list,
        );

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        self.frame_index = self.frame_index.saturating_add(1);
        let stats = FrameStats {
            frame_index: self.frame_index,
            width: self.surface_config.width,
            height: self.surface_config.height,
            draw_commands: self.draw_list.len(),
            cpu_frame_ms: start.elapsed().as_secs_f64() * 1000.0,
        };
        Ok(stats)
    }
}

// Surface-independent draw path, also used by offscreen GPU regression tests.
struct UiRenderer {
    rect_pipeline: wgpu::RenderPipeline,
    image_pipeline: wgpu::RenderPipeline,
    image_bind_group_layout: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    image_cache: HashMap<ImageCacheKey, CachedImage>,
    rect_buffers: VertexBufferPool,
    rect_scratch: Vec<UiVertex>,
    image_scratch: Vec<u8>,
    frame_index: u64,
}

impl UiRenderer {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let rect_pipeline = create_rect_pipeline(device, format);
        let image_bind_group_layout = create_image_bind_group_layout(device);
        let image_sampler = create_image_sampler(device);
        let image_pipeline = create_image_pipeline(device, format, &image_bind_group_layout);

        Self {
            rect_pipeline,
            image_pipeline,
            image_bind_group_layout,
            image_sampler,
            image_cache: HashMap::new(),
            rect_buffers: VertexBufferPool::default(),
            rect_scratch: Vec::new(),
            image_scratch: Vec::new(),
            frame_index: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: RenderSize,
        ui_size: Size,
        draw_list: &DrawList,
    ) {
        self.frame_index = self.frame_index.saturating_add(1);
        // Keep only current/recent images. Changing frame-counter text must not
        // accumulate textures indefinitely or trigger whole-cache eviction.
        self.image_cache
            .retain(|_, image| image.last_used_frame.saturating_add(2) >= self.frame_index);
        self.rect_buffers.next = 0;
        let mut rect_vertices = std::mem::take(&mut self.rect_scratch);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("plita.phase1.ui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            ClearColor::from(draw_list.clear_color).to_wgpu(),
                        ),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let surface_width = size.width as f32;
            let surface_height = size.height as f32;
            let width = ui_size.width.max(1.0);
            let height = ui_size.height.max(1.0);
            let full_clip = Rect::new(0.0, 0.0, width, height);
            let mut clip_stack = vec![full_clip];
            for command in draw_list.commands().iter().cloned() {
                match command {
                    DrawCommand::FillRect { rect, color } => {
                        append_rect_vertices(&mut rect_vertices, rect, color, width, height);
                    }
                    DrawCommand::FillRectCorners {
                        rect,
                        top_left,
                        top_right,
                        bottom_right,
                        bottom_left,
                    } => {
                        append_rect_corner_vertices(
                            &mut rect_vertices,
                            rect,
                            top_left,
                            top_right,
                            bottom_right,
                            bottom_left,
                            width,
                            height,
                        );
                    }
                    DrawCommand::FillTriangles { vertices } => {
                        append_colored_triangle_vertices(
                            &mut rect_vertices,
                            &vertices,
                            width,
                            height,
                        );
                    }
                    DrawCommand::ImageRgba {
                        rect,
                        width: image_width,
                        height: image_height,
                        pixels,
                    } => {
                        flush_rect_vertices(
                            &mut render_pass,
                            device,
                            queue,
                            &mut self.rect_buffers,
                            &self.rect_pipeline,
                            &mut rect_vertices,
                        );
                        draw_image_rgba(
                            &mut render_pass,
                            device,
                            queue,
                            &self.image_pipeline,
                            &self.image_bind_group_layout,
                            &self.image_sampler,
                            &mut self.image_cache,
                            rect,
                            image_width,
                            image_height,
                            ImageSource::Immutable(Arc::clone(&pixels)),
                            self.frame_index,
                            &pixels,
                            width,
                            height,
                        );
                    }
                    DrawCommand::ImageRgbaShared {
                        rect,
                        width: image_width,
                        height: image_height,
                        pixels,
                    } => {
                        flush_rect_vertices(
                            &mut render_pass,
                            device,
                            queue,
                            &mut self.rect_buffers,
                            &self.rect_pipeline,
                            &mut rect_vertices,
                        );
                        let snapshot_ready = if let Ok(locked) = pixels.lock() {
                            self.image_scratch.clear();
                            self.image_scratch.extend_from_slice(&locked);
                            true
                        } else {
                            false
                        };
                        // Release the producer lock before GPU allocation/upload.
                        if snapshot_ready {
                            draw_image_rgba(
                                &mut render_pass,
                                device,
                                queue,
                                &self.image_pipeline,
                                &self.image_bind_group_layout,
                                &self.image_sampler,
                                &mut self.image_cache,
                                rect,
                                image_width,
                                image_height,
                                ImageSource::Shared(Arc::clone(&pixels)),
                                self.frame_index,
                                &self.image_scratch,
                                width,
                                height,
                            );
                        }
                    }
                    DrawCommand::PushClip { rect } => {
                        flush_rect_vertices(
                            &mut render_pass,
                            device,
                            queue,
                            &mut self.rect_buffers,
                            &self.rect_pipeline,
                            &mut rect_vertices,
                        );
                        let parent = clip_stack.last().copied().unwrap_or(full_clip);
                        let next =
                            intersect_rect(parent, rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
                        clip_stack.push(next);
                        set_scissor_rect(
                            &mut render_pass,
                            next,
                            width,
                            height,
                            surface_width,
                            surface_height,
                        );
                    }
                    DrawCommand::PopClip => {
                        flush_rect_vertices(
                            &mut render_pass,
                            device,
                            queue,
                            &mut self.rect_buffers,
                            &self.rect_pipeline,
                            &mut rect_vertices,
                        );
                        if clip_stack.len() > 1 {
                            clip_stack.pop();
                        }
                        let current = clip_stack.last().copied().unwrap_or(full_clip);
                        set_scissor_rect(
                            &mut render_pass,
                            current,
                            width,
                            height,
                            surface_width,
                            surface_height,
                        );
                    }
                }
            }
            flush_rect_vertices(
                &mut render_pass,
                device,
                queue,
                &mut self.rect_buffers,
                &self.rect_pipeline,
                &mut rect_vertices,
            );
        }
        self.rect_scratch = rect_vertices;
        self.rect_buffers.buffers.truncate(self.rect_buffers.next);
    }
}

fn plita_instance_descriptor() -> wgpu::InstanceDescriptor {
    let mut descriptor = wgpu::InstanceDescriptor {
        backends: wgpu::Instance::enabled_backend_features(),
        ..Default::default()
    }
    .with_env();

    if descriptor.backends.is_empty() {
        warn!(
            compiled_backends = ?wgpu::Instance::enabled_backend_features(),
            "WGPU backend selection was empty; falling back to compiled native backends"
        );
        descriptor.backends = wgpu::Instance::enabled_backend_features();
    }

    descriptor
}

fn create_rect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("plita.phase1.ui_rects.shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_rects.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("plita.phase1.ui_rects.pipeline_layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("plita.phase1.ui_rects.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[UiVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_image_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("plita.phase2.image.bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_image_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("plita.phase2.image.nearest_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

fn create_image_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    image_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("plita.phase2.image.shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_textured.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("plita.phase2.image.pipeline_layout"),
        bind_group_layouts: &[image_bind_group_layout],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("plita.phase2.image.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[ImageVertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[derive(Default)]
struct VertexBufferPool {
    buffers: Vec<(wgpu::Buffer, u64)>,
    next: usize,
}

impl VertexBufferPool {
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> &wgpu::Buffer {
        let length = bytes.len() as u64;
        let slot = self.next;
        self.next += 1;
        if self
            .buffers
            .get(slot)
            .is_none_or(|(_, capacity)| *capacity < length)
        {
            let capacity = length.next_power_of_two();
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("plita.ui.reusable_vertices"),
                size: capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            if slot == self.buffers.len() {
                self.buffers.push((buffer, capacity));
            } else {
                self.buffers[slot] = (buffer, capacity);
            }
        }
        let buffer = &self.buffers[slot].0;
        queue.write_buffer(buffer, 0, bytes);
        buffer
    }
}

fn flush_rect_vertices(
    render_pass: &mut wgpu::RenderPass<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffers: &mut VertexBufferPool,
    rect_pipeline: &wgpu::RenderPipeline,
    vertices: &mut Vec<UiVertex>,
) {
    if vertices.is_empty() {
        return;
    }

    let vertex_buffer = buffers.upload(device, queue, bytemuck::cast_slice(vertices));
    render_pass.set_pipeline(rect_pipeline);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..vertices.len() as u32, 0..1);
    vertices.clear();
}

fn set_scissor_rect(
    render_pass: &mut wgpu::RenderPass<'_>,
    rect: Rect,
    width: f32,
    height: f32,
    surface_width: f32,
    surface_height: f32,
) {
    let scale_x = surface_width / width.max(1.0);
    let scale_y = surface_height / height.max(1.0);
    let x = (rect.x.clamp(0.0, width) * scale_x).floor() as u32;
    let y = (rect.y.clamp(0.0, height) * scale_y).floor() as u32;
    let right = (rect.right().clamp(0.0, width) * scale_x).ceil() as u32;
    let bottom = (rect.bottom().clamp(0.0, height) * scale_y).ceil() as u32;
    let w = right.saturating_sub(x).max(1);
    let h = bottom.saturating_sub(y).max(1);
    render_pass.set_scissor_rect(x, y, w, h);
}

fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.right().min(b.right());
    let y1 = a.bottom().min(b.bottom());
    (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

#[allow(clippy::too_many_arguments)]
fn draw_image_rgba(
    render_pass: &mut wgpu::RenderPass<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image_pipeline: &wgpu::RenderPipeline,
    image_bind_group_layout: &wgpu::BindGroupLayout,
    image_sampler: &wgpu::Sampler,
    image_cache: &mut HashMap<ImageCacheKey, CachedImage>,
    rect: Rect,
    image_width: u32,
    image_height: u32,
    source: ImageSource,
    frame_index: u64,
    pixels: &[u8],
    surface_width: f32,
    surface_height: f32,
) {
    let expected_len = image_width as usize * image_height as usize * 4;
    if image_width == 0 || image_height == 0 || pixels.len() != expected_len {
        return;
    }

    let size = wgpu::Extent3d {
        width: image_width,
        height: image_height,
        depth_or_array_layers: 1,
    };
    let (slot, shared) = source.identity();
    let cache_key = ImageCacheKey {
        slot,
        shared,
        len: pixels.len(),
        width: image_width,
        height: image_height,
    };

    let created = !image_cache.contains_key(&cache_key);
    if created {
        let cached = create_cached_image(
            device,
            queue,
            image_bind_group_layout,
            image_sampler,
            size,
            pixels,
            source,
            frame_index,
        );
        image_cache.insert(cache_key, cached);
    }

    let Some(cached) = image_cache.get_mut(&cache_key) else {
        return;
    };
    if shared && !created {
        update_cached_image(queue, cached, size, pixels);
    }
    cached.last_used_frame = frame_index;

    let vertex_key = ImageVertexCacheKey {
        rect: [
            rect.x.to_bits(),
            rect.y.to_bits(),
            rect.width.to_bits(),
            rect.height.to_bits(),
        ],
        surface_width: surface_width.max(1.0).to_bits(),
        surface_height: surface_height.max(1.0).to_bits(),
    };
    if cached.vertex_key != Some(vertex_key) {
        let mut vertices = Vec::with_capacity(6);
        append_image_vertices(
            &mut vertices,
            rect,
            surface_width.max(1.0),
            surface_height.max(1.0),
        );
        if vertices.is_empty() {
            cached.vertex_key = None;
            cached.vertex_buffer = None;
            cached.vertex_count = 0;
            return;
        }
        cached.vertex_count = vertices.len() as u32;
        cached.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("plita.phase2.image.vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
        cached.vertex_key = Some(vertex_key);
    }
    let Some(vertex_buffer) = cached.vertex_buffer.as_ref() else {
        return;
    };
    if cached.vertex_count == 0 {
        return;
    }

    render_pass.set_pipeline(image_pipeline);
    render_pass.set_bind_group(0, &cached.bind_group, &[]);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.draw(0..cached.vertex_count, 0..1);
}

fn create_cached_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image_bind_group_layout: &wgpu::BindGroupLayout,
    image_sampler: &wgpu::Sampler,
    size: wgpu::Extent3d,
    pixels: &[u8],
    source: ImageSource,
    frame_index: u64,
) -> CachedImage {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("plita.phase2.image.rgba_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        size,
    );

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("plita.phase2.image.bind_group"),
        layout: image_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(image_sampler),
            },
        ],
    });
    CachedImage {
        _source: source,
        last_used_frame: frame_index,
        texture,
        _view: texture_view,
        bind_group,
        vertex_key: None,
        vertex_buffer: None,
        vertex_count: 0,
    }
}

fn update_cached_image(
    queue: &wgpu::Queue,
    cached: &CachedImage,
    size: wgpu::Extent3d,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &cached.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        size,
    );
}

#[cfg(test)]
fn build_ui_vertices(draw_list: &DrawList, width: f32, height: f32) -> Vec<UiVertex> {
    let width = width.max(1.0);
    let height = height.max(1.0);
    let mut vertices = Vec::with_capacity(draw_list.len() * 6);

    for command in draw_list.commands() {
        match command {
            DrawCommand::FillRect { rect, color } => {
                append_rect_vertices(&mut vertices, *rect, *color, width, height);
            }
            DrawCommand::FillRectCorners {
                rect,
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => append_rect_corner_vertices(
                &mut vertices,
                *rect,
                *top_left,
                *top_right,
                *bottom_right,
                *bottom_left,
                width,
                height,
            ),
            DrawCommand::FillTriangles {
                vertices: mesh_vertices,
            } => append_colored_triangle_vertices(&mut vertices, mesh_vertices, width, height),
            DrawCommand::ImageRgba { .. } => {}
            DrawCommand::ImageRgbaShared { .. } => {}
            DrawCommand::PushClip { .. } | DrawCommand::PopClip => {}
        }
    }

    vertices
}

fn append_rect_vertices(
    vertices: &mut Vec<UiVertex>,
    rect: Rect,
    color: Color,
    width: f32,
    height: f32,
) {
    append_rect_corner_vertices(vertices, rect, color, color, color, color, width, height);
}

fn append_rect_corner_vertices(
    vertices: &mut Vec<UiVertex>,
    rect: Rect,
    top_left: Color,
    top_right: Color,
    bottom_right: Color,
    bottom_left: Color,
    width: f32,
    height: f32,
) {
    let x0 = rect.x.clamp(0.0, width);
    let y0 = rect.y.clamp(0.0, height);
    let x1 = rect.right().clamp(0.0, width);
    let y1 = rect.bottom().clamp(0.0, height);

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let left = x0 / width * 2.0 - 1.0;
    let right = x1 / width * 2.0 - 1.0;
    let top = 1.0 - y0 / height * 2.0;
    let bottom = 1.0 - y1 / height * 2.0;
    let top_left = [top_left.r, top_left.g, top_left.b, top_left.a];
    let top_right = [top_right.r, top_right.g, top_right.b, top_right.a];
    let bottom_right = [
        bottom_right.r,
        bottom_right.g,
        bottom_right.b,
        bottom_right.a,
    ];
    let bottom_left = [bottom_left.r, bottom_left.g, bottom_left.b, bottom_left.a];

    vertices.extend_from_slice(&[
        UiVertex {
            position: [left, top],
            color: top_left,
        },
        UiVertex {
            position: [left, bottom],
            color: bottom_left,
        },
        UiVertex {
            position: [right, bottom],
            color: bottom_right,
        },
        UiVertex {
            position: [left, top],
            color: top_left,
        },
        UiVertex {
            position: [right, bottom],
            color: bottom_right,
        },
        UiVertex {
            position: [right, top],
            color: top_right,
        },
    ]);
}

fn append_colored_triangle_vertices(
    vertices: &mut Vec<UiVertex>,
    source: &[ColoredVertex],
    width: f32,
    height: f32,
) {
    for vertex in source {
        if vertex.position.x.is_finite()
            && vertex.position.y.is_finite()
            && vertex.color.r.is_finite()
            && vertex.color.g.is_finite()
            && vertex.color.b.is_finite()
            && vertex.color.a.is_finite()
        {
            vertices.push(UiVertex {
                position: point_to_clip(vertex.position, width, height),
                color: [
                    vertex.color.r,
                    vertex.color.g,
                    vertex.color.b,
                    vertex.color.a,
                ],
            });
        }
    }
}

fn point_to_clip(point: plita_ui::Point, width: f32, height: f32) -> [f32; 2] {
    [
        point.x / width.max(1.0) * 2.0 - 1.0,
        1.0 - point.y / height.max(1.0) * 2.0,
    ]
}

fn append_image_vertices(vertices: &mut Vec<ImageVertex>, rect: Rect, width: f32, height: f32) {
    let x0 = rect.x.clamp(0.0, width);
    let y0 = rect.y.clamp(0.0, height);
    let x1 = rect.right().clamp(0.0, width);
    let y1 = rect.bottom().clamp(0.0, height);

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let left = x0 / width * 2.0 - 1.0;
    let right = x1 / width * 2.0 - 1.0;
    let top = 1.0 - y0 / height * 2.0;
    let bottom = 1.0 - y1 / height * 2.0;

    vertices.extend_from_slice(&[
        ImageVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        ImageVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
        ImageVertex {
            position: [right, bottom],
            uv: [1.0, 1.0],
        },
        ImageVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
        ImageVertex {
            position: [right, bottom],
            uv: [1.0, 1.0],
        },
        ImageVertex {
            position: [right, top],
            uv: [1.0, 0.0],
        },
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn prepared_native_surface_can_move_to_gpu_worker() {
        fn assert_send<T: Send>() {}
        assert_send::<PreparedSurface>();
    }

    #[test]
    fn render_size_clamps_zero() {
        assert_eq!(
            RenderSize::clamped(0, 0),
            RenderSize {
                width: 1,
                height: 1
            }
        );
        assert_eq!(
            RenderSize::clamped(640, 0),
            RenderSize {
                width: 640,
                height: 1
            }
        );
    }

    #[test]
    fn render_size_converts_physical_pixels_to_logical_ui_units() {
        let size = RenderSize::clamped(1_560, 1_080).to_ui_size_with_scale_factor(1.5);
        assert_eq!(size, Size::new(1_040.0, 720.0));
    }

    #[test]
    fn render_size_rejects_invalid_scale_factors() {
        let physical = RenderSize::clamped(640, 480);
        assert_eq!(
            physical.to_ui_size_with_scale_factor(0.0),
            Size::new(640.0, 480.0)
        );
        assert_eq!(
            physical.to_ui_size_with_scale_factor(f64::NAN),
            Size::new(640.0, 480.0)
        );
    }

    #[test]
    fn clear_color_conversion_preserves_channels() {
        let color = ClearColor::rgba(0.1, 0.2, 0.3, 0.4).to_wgpu();
        assert_eq!(color.r, 0.1);
        assert_eq!(color.g, 0.2);
        assert_eq!(color.b, 0.3);
        assert_eq!(color.a, 0.4);
    }

    #[test]
    fn vertex_builder_creates_two_triangles_per_rect() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.fill_rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::rgb8(255, 255, 255));
        let vertices = build_ui_vertices(&list, 200.0, 100.0);
        assert_eq!(vertices.len(), 6);
    }

    #[test]
    fn vertex_builder_preserves_rect_corner_colors() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.fill_rect_corners(
            Rect::new(0.0, 0.0, 100.0, 50.0),
            Color::rgba(1.0, 0.0, 0.0, 1.0),
            Color::rgba(0.0, 1.0, 0.0, 1.0),
            Color::rgba(0.0, 0.0, 1.0, 1.0),
            Color::rgba(1.0, 1.0, 0.0, 1.0),
        );
        let vertices = build_ui_vertices(&list, 200.0, 100.0);

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[1].color, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(vertices[2].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(vertices[3].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[4].color, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(vertices[5].color, [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn vertex_builder_preserves_triangle_mesh_vertices() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.fill_triangle_list(vec![
            ColoredVertex::new(
                plita_ui::Point::new(0.0, 0.0),
                Color::rgba(1.0, 0.0, 0.0, 1.0),
            ),
            ColoredVertex::new(
                plita_ui::Point::new(100.0, 0.0),
                Color::rgba(0.0, 1.0, 0.0, 1.0),
            ),
            ColoredVertex::new(
                plita_ui::Point::new(0.0, 50.0),
                Color::rgba(0.0, 0.0, 1.0, 1.0),
            ),
        ]);
        let vertices = build_ui_vertices(&list, 200.0, 100.0);

        assert_eq!(vertices.len(), 3);
        assert_eq!(vertices[0].position, [-1.0, 1.0]);
        assert_eq!(vertices[1].position, [0.0, 1.0]);
        assert_eq!(vertices[2].position, [-1.0, 0.0]);
        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[1].color, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(vertices[2].color, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn vertex_builder_ignores_textured_commands_for_rect_pass() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.image_rgba8(Rect::new(0.0, 0.0, 100.0, 50.0), 2, 2, vec![255; 16]);
        let vertices = build_ui_vertices(&list, 200.0, 100.0);
        assert!(vertices.is_empty());
    }

    #[test]
    fn vertex_builder_ignores_clip_commands_for_rect_pass() {
        let mut list = DrawList::new(Color::rgb8(0, 0, 0));
        list.push_clip(Rect::new(4.0, 4.0, 20.0, 20.0));
        list.pop_clip();
        let vertices = build_ui_vertices(&list, 200.0, 100.0);
        assert!(vertices.is_empty());
    }

    #[test]
    fn image_vertex_builder_creates_two_triangles() {
        let mut vertices = Vec::new();
        append_image_vertices(
            &mut vertices,
            Rect::new(0.0, 0.0, 100.0, 50.0),
            200.0,
            100.0,
        );

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn instance_descriptor_enables_compiled_native_backends() {
        let descriptor = plita_instance_descriptor();
        let compiled = wgpu::Instance::enabled_backend_features();

        assert!(!compiled.is_empty());
        assert!(!descriptor.backends.is_empty());
        assert!(descriptor.backends.intersects(compiled));
    }
}

#[cfg(test)]
mod gpu_tests;
