//! Explicit opt-in tests: exercise the production GPU path without a window.
use super::*;
use std::{path::PathBuf, sync::Mutex};

struct Offscreen {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: UiRenderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: RenderSize,
    adapter: String,
}

impl Offscreen {
    fn new(width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(&plita_instance_descriptor());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("GPU adapter required for explicitly requested GPU test");
        let adapter_name = format!("{:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("plita.offscreen.test"),
            ..Default::default()
        }))
        .unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let renderer = UiRenderer::new(&device, format);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("plita.offscreen.target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            device,
            queue,
            renderer,
            texture,
            view,
            size: RenderSize::clamped(width, height),
            adapter: adapter_name,
        }
    }

    fn render(&mut self, list: &DrawList) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.renderer.encode(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.view,
            self.size,
            self.size.to_ui_size(),
            list,
        );
        self.queue.submit(Some(encoder.finish()));
    }

    fn wait(&self) {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
    }

    fn read(&self) -> Vec<u8> {
        let stride = (self.size.width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("plita.offscreen.readback"),
            size: u64::from(stride * self.size.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(self.size.height),
                },
            },
            self.texture.size(),
        );
        self.queue.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
        self.wait();
        rx.recv().unwrap().unwrap();
        let mapped = buffer.slice(..).get_mapped_range();
        mapped
            .chunks_exact(stride as usize)
            .flat_map(|row| row[..self.size.width as usize * 4].iter().copied())
            .collect()
    }
}

fn exported_list(path: &std::path::Path) -> (DrawList, u32, u32) {
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let color = |v: &serde_json::Value| {
        Color::rgba(
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
            v[3].as_f64().unwrap() as f32,
        )
    };
    let rect = |v: &serde_json::Value| {
        Rect::new(
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
            v[3].as_f64().unwrap() as f32,
        )
    };
    let mut list = DrawList::new(color(&data["background"]));
    for c in data["commands"].as_array().unwrap() {
        match c["kind"].as_str().unwrap() {
            "rect" => list.fill_rect(rect(&c["rect"]), color(&c["color"])),
            "triangles" => list.fill_triangle_list(
                c["vertices"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| ColoredVertex {
                        position: plita_ui::Point::new(
                            v["point"][0].as_f64().unwrap() as f32,
                            v["point"][1].as_f64().unwrap() as f32,
                        ),
                        color: color(&v["color"]),
                    })
                    .collect::<Vec<_>>(),
            ),
            "image" => {
                let width = c["width"].as_u64().unwrap() as u32;
                let height = c["height"].as_u64().unwrap() as u32;
                let pixels: Vec<u8> = c["pixels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_u64().unwrap() as u8)
                    .collect();
                if c["shared"]
                    .as_bool()
                    .unwrap_or(width == 256 && height == 240)
                {
                    list.image_rgba8_shared(
                        rect(&c["rect"]),
                        width,
                        height,
                        Arc::new(Mutex::new(pixels)),
                    );
                } else {
                    list.image_rgba8(rect(&c["rect"]), width, height, pixels);
                }
            }
            "push_clip" => list.push_clip(rect(&c["rect"])),
            "pop_clip" => list.pop_clip(),
            other => panic!("unsupported evidence command {other}"),
        }
    }
    (
        list,
        data["width"].as_f64().unwrap() as u32,
        data["height"].as_f64().unwrap() as u32,
    )
}

#[test]
#[ignore = "requires GPU and PLITA_GPU_EVIDENCE / PLITA_DRAW_FIXTURE"]
fn offscreen_desktop_benchmark() {
    let root = PathBuf::from(std::env::var_os("PLITA_GPU_EVIDENCE").unwrap());
    let fixture = PathBuf::from(std::env::var_os("PLITA_DRAW_FIXTURE").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let (list, width, height) = exported_list(&fixture);
    let mut gpu = Offscreen::new(width, height);
    for _ in 0..30 {
        gpu.render(&list);
    }
    gpu.wait();
    let mut times = Vec::new();
    // Wait after each submission: no growing GPU queue or inflated throughput.
    for index in 0..180 {
        // A changing counter and video buffer model a running GUI rather than
        // measuring only an entirely static cached screen.
        let mut frame_list = list.clone();
        plita_ui::draw_text(
            &mut frame_list,
            980.,
            780.,
            &format!("Frame {index}"),
            1.,
            Color::rgb(1., 1., 1.),
        );
        for command in frame_list.commands() {
            if let DrawCommand::ImageRgbaShared { pixels, .. } = command {
                pixels.lock().unwrap()[0] = index as u8;
            }
        }
        let start = Instant::now();
        gpu.render(&frame_list);
        gpu.wait();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let result = serde_json::json!({
        "adapter": gpu.adapter, "fixture": fixture, "width": width, "height": height,
        "draw_commands": list.len(), "frames": times.len(),
        "median_ms": times[times.len()/2], "p95_ms": times[times.len()*95/100],
        "mean_ms": times.iter().sum::<f64>() / times.len() as f64,
        "method": "production plita GPU draw path; offscreen; includes submit and GPU completion; excludes window present and emulation",
    });
    std::fs::write(
        root.join("gpu.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    std::fs::write(root.join("desktop.rgba"), gpu.read()).unwrap();
    println!("{result}");
}

#[test]
#[ignore = "set PLITA_DRAW_FIXTURE and PLITA_GPU_EVIDENCE for changing emulator images"]
fn offscreen_emulator_stream_benchmark() {
    let root = PathBuf::from(std::env::var_os("PLITA_GPU_EVIDENCE").unwrap());
    let fixture = PathBuf::from(std::env::var_os("PLITA_DRAW_FIXTURE").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let (base, width, height) = exported_list(&fixture);
    assert_eq!(
        base.commands()
            .iter()
            .filter(|c| matches!(c, DrawCommand::ImageRgbaShared { .. }))
            .count(),
        1
    );
    let order = if std::env::var_os("PLITA_PERF_SHARED_FIRST").is_some() {
        [true, false]
    } else {
        [false, true]
    };
    let mut reference = None;
    let mut results = Vec::new();
    for shared in order {
        let mut gpu = Offscreen::new(width, height);
        let mut times = Vec::new();
        let mut allocations = 0;
        for frame in 0..210 {
            let start = Instant::now();
            let mut list = DrawList::new(base.clear_color);
            for command in base.commands() {
                if let DrawCommand::ImageRgbaShared {
                    rect,
                    width,
                    height,
                    pixels,
                } = command
                {
                    pixels.lock().unwrap()[0] = frame as u8;
                    let source = if shared {
                        list.push(command.clone());
                        ImageSource::Shared(pixels.clone())
                    } else {
                        let snapshot: Arc<[u8]> = pixels.lock().unwrap().clone().into();
                        list.image_rgba8(*rect, *width, *height, snapshot.clone());
                        ImageSource::Immutable(snapshot)
                    };
                    let (slot, is_shared) = source.identity();
                    let key = ImageCacheKey {
                        slot,
                        shared: is_shared,
                        len: (width * height * 4) as usize,
                        width: *width,
                        height: *height,
                    };
                    if frame >= 30 && !gpu.renderer.image_cache.contains_key(&key) {
                        allocations += 1;
                    }
                } else {
                    list.push(command.clone());
                }
            }
            gpu.render(&list);
            gpu.wait();
            if frame >= 30 {
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        times.sort_by(f64::total_cmp);
        let pixels = gpu.read();
        if let Some(ref expected) = reference {
            assert_eq!(&pixels, expected, "snapshot/shared GPU output");
        } else {
            reference = Some(pixels.clone());
        }
        assert_eq!(allocations, if shared { 0 } else { 180 });
        let variant = if shared { "shared" } else { "snapshot" };
        std::fs::write(root.join(format!("{variant}.rgba")), pixels).unwrap();
        results.push(serde_json::json!({"variant":variant,"adapter":gpu.adapter,"frames":times.len(),"median_ms":times[times.len()/2],"p95_ms":times[times.len()*95/100],"mean_ms":times.iter().sum::<f64>()/times.len() as f64,"video_texture_allocations_after_warmup":allocations}));
    }
    let result = serde_json::json!({"fixture":fixture,"width":width,"height":height,"pixels_identical":true,"samples":results,"method":"same production renderer, actual emulator DrawList with changing video pixels; previous owned snapshots versus shared video texture; includes list handoff, submit and GPU completion; excludes emulation and window present"});
    std::fs::write(
        root.join("gpu.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!("{result}");
}

#[test]
#[ignore = "requires a GPU adapter; renders and reads pixels"]
fn offscreen_image_updates_and_draw_order() {
    let mut gpu = Offscreen::new(64, 64);
    let shared = Arc::new(Mutex::new(vec![255, 0, 0, 255]));
    let mut list = DrawList::new(Color::rgb(0., 0., 0.));
    list.image_rgba8_shared(Rect::new(0., 0., 32., 32.), 1, 1, shared.clone());
    let white: Arc<[u8]> = vec![255; 4].into();
    list.image_rgba8(Rect::new(32., 0., 32., 32.), 1, 1, white.clone());
    list.image_rgba8(Rect::new(0., 32., 32., 32.), 1, 1, white);
    list.push_clip(Rect::new(8., 8., 8., 8.));
    list.fill_rect(Rect::new(0., 0., 64., 64.), Color::rgb(0., 0., 1.));
    list.pop_clip();
    for expected in [[255, 0, 0, 255], [0, 255, 0, 255]] {
        shared.lock().unwrap().copy_from_slice(&expected);
        gpu.render(&list);
        let pixels = gpu.read();
        let pixel = |x: usize, y: usize| &pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
        assert_eq!(pixel(4, 4), expected);
        assert_eq!(pixel(12, 12), [0, 0, 255, 255]);
        assert_eq!(pixel(40, 4), [255; 4]);
        assert_eq!(pixel(4, 40), [255; 4]);
        assert_eq!(pixel(40, 40), [0, 0, 0, 255]);
    }
}

#[test]
#[ignore = "requires a GPU adapter; tests retained textures and buffer resizing"]
fn offscreen_cache_keeps_images_distinct_and_releases_old_sources() {
    let mut gpu = Offscreen::new(64, 64);
    let mut source: Arc<[u8]> = vec![255, 0, 0, 255].into();
    let original = Arc::downgrade(&source);
    for index in 0..12 {
        let expected = [index * 20, 255 - index * 20, 0, 255];
        Arc::make_mut(&mut source).copy_from_slice(&expected);
        let mut list = DrawList::new(Color::rgb(0., 0., 0.));
        list.image_rgba8(Rect::new(0., 0., 32., 32.), 1, 1, source.clone());
        // Changing batch sizes catches reuse of an undersized vertex buffer.
        for row in 0..=index {
            list.fill_rect(
                Rect::new(32., row as f32 * 2., 32., 2.),
                Color::rgb(0., 0., 1.),
            );
        }
        list.image_rgba8(Rect::new(0., 32., 32., 32.), 1, 1, source.clone());
        list.fill_rect(Rect::new(32., 32., 32., 32.), Color::rgb(1., 1., 1.));
        gpu.render(&list);
        let pixels = gpu.read();
        assert_eq!(&pixels[0..4], expected);
        assert!(Arc::strong_count(&source) >= 3);
        assert_eq!(&pixels[32 * 64 * 4..32 * 64 * 4 + 4], expected);
        assert_eq!(
            &pixels[(40 * 64 + 40) * 4..(40 * 64 + 40) * 4 + 4],
            [255; 4]
        );
        assert!(gpu.renderer.image_cache.len() <= 3);
    }
    assert!(original.upgrade().is_none());
    let last = Arc::downgrade(&source);
    drop(source);
    for _ in 0..3 {
        gpu.render(&DrawList::new(Color::rgb(0., 0., 0.)));
    }
    gpu.wait();
    assert!(last.upgrade().is_none());
    assert!(gpu.renderer.image_cache.is_empty());
}
