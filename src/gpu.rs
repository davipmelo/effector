//! GPU layer (wgpu): shared device, compiled compute kernels, texture pool and the per-render
//! context used by effects. Mirrors `GPU` / `RenderContext` of the original macOS app: every
//! kernel writes texture binding 0, reads inputs at 7.., floats `u` (binding 1) and `ex` (binding 2).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use wgpu::util::DeviceExt;

include!(concat!(env!("OUT_DIR"), "/kernels.rs"));

const HEADER: &str = include_str!("shaders/header.glsl");
const PRELUDE: &str = include_str!("shaders/prelude.glsl");

pub const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// A GPU image. Shared with `Arc` so identity checks (`Arc::ptr_eq`) work like `!==` in Swift.
pub struct Tex {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub w: u32,
    pub h: u32,
    pub d: u32,
    pub format: wgpu::TextureFormat,
}
pub type T = Arc<Tex>;

impl Tex {
    pub fn width(&self) -> usize { self.w as usize }
    pub fn height(&self) -> usize { self.h as usize }
}

struct Kernel {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// (binding, is3d) for inputs texture(1), texture(2)...
    inputs: Vec<(u32, bool)>,
}

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    kernels: Mutex<HashMap<String, Arc<Kernel>>>,
    samplers: [wgpu::Sampler; 4],
    dummy2d: T,
    dummy3d: T,
    /// rgba32float textures can be linearly filtered (else lookup tables fall back to rgba16float).
    pub f32_filterable: bool,
    pub max_dim: u32,
}

static GPU: OnceLock<Gpu> = OnceLock::new();

thread_local! {
    /// Recycled working textures. Per thread: a texture freed by one thread may still be read by that
    /// thread's not-yet-submitted commands, so only the same thread (same queue order) may reuse it.
    static POOL: std::cell::RefCell<Vec<T>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn gpu() -> &'static Gpu {
    GPU.get().expect("GPU not initialised")
}

pub fn is_ready() -> bool { GPU.get().is_some() }

/// Features/limits the app asks for (used both by the window and headless modes).
pub fn wanted_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    adapter.features() & wgpu::Features::FLOAT32_FILTERABLE
}

pub fn wanted_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    let a = adapter.limits();
    wgpu::Limits {
        max_texture_dimension_2d: a.max_texture_dimension_2d,
        max_texture_dimension_1d: a.max_texture_dimension_1d,
        max_buffer_size: a.max_buffer_size,
        max_storage_buffer_binding_size: a.max_storage_buffer_binding_size,
        max_storage_buffers_per_shader_stage: a.max_storage_buffers_per_shader_stage.min(16),
        max_sampled_textures_per_shader_stage: a.max_sampled_textures_per_shader_stage.min(16),
        ..wgpu::Limits::default()
    }
}

/// Initialise from an existing device (the window's) or create a headless one.
pub fn init(device: wgpu::Device, queue: wgpu::Queue) {
    // wgpu's default handler panics on any validation / out-of-memory error, which killed the
    // render thread (the preview then froze for good). Log the error and keep going instead.
    device.on_uncaptured_error(Arc::new(|e: wgpu::Error| {
        crate::settings::log_error(&format!("GPU error: {e}"));
    }));
    let _ = GPU.set(Gpu::new(device, queue));
}

pub fn init_headless() -> Result<(), String> {
    if is_ready() { return Ok(()); }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|e| format!("no GPU adapter: {e}"))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("effector"),
        required_features: wanted_features(&adapter),
        required_limits: wanted_limits(&adapter),
        ..Default::default()
    }))
    .map_err(|e| format!("device: {e}"))?;
    init(device, queue);
    Ok(())
}

fn parse_inputs(src: &str) -> Vec<(u32, bool)> {
    let mut v = vec![];
    for line in src.lines() {
        let l = line.trim();
        if !l.starts_with("layout(set = 0, binding = ") { continue; }
        let is3 = l.contains("texture3D");
        if !(l.contains("texture2D") || is3) { continue; }
        let n: u32 = l["layout(set = 0, binding = ".len()..].split(')').next().unwrap().trim().parse().unwrap();
        v.push((n, is3));
    }
    v.sort();
    v
}

pub fn kernel_source(name: &str) -> Option<String> {
    KERNELS.iter().find(|(n, _)| *n == name).map(|(_, b)| format!("{HEADER}\n{PRELUDE}\n{b}"))
}

pub fn kernel_names() -> Vec<&'static str> { KERNELS.iter().map(|(n, _)| *n).collect() }

impl Gpu {
    fn new(device: wgpu::Device, queue: wgpu::Queue) -> Gpu {
        let mk = |filter: wgpu::FilterMode, addr: wgpu::AddressMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: addr,
                address_mode_v: addr,
                address_mode_w: addr,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let samplers = [
            mk(wgpu::FilterMode::Linear, wgpu::AddressMode::ClampToEdge),
            mk(wgpu::FilterMode::Nearest, wgpu::AddressMode::ClampToEdge),
            mk(wgpu::FilterMode::Linear, wgpu::AddressMode::Repeat),
            mk(wgpu::FilterMode::Linear, wgpu::AddressMode::MirrorRepeat),
        ];
        let f32_filterable = device.features().contains(wgpu::Features::FLOAT32_FILTERABLE);
        let max_dim = device.limits().max_texture_dimension_2d;
        let make_dummy = |dim: wgpu::TextureDimension| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("dummy"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: dim,
                format: FMT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            Arc::new(Tex { texture, view, w: 1, h: 1, d: 1, format: FMT })
        };
        let dummy2d = make_dummy(wgpu::TextureDimension::D2);
        let dummy3d = make_dummy(wgpu::TextureDimension::D3);
        Gpu {
            device,
            queue,
            kernels: Mutex::new(HashMap::new()),
            samplers,
            dummy2d,
            dummy3d,
            f32_filterable,
            max_dim,
        }
    }

    fn kernel(&self, name: &str) -> Arc<Kernel> {
        if let Some(k) = self.kernels.lock().unwrap().get(name) {
            return k.clone();
        }
        let src = kernel_source(name).unwrap_or_else(|| panic!("Missing kernel {name}"));
        let mut fe = naga::front::glsl::Frontend::default();
        let module = fe
            .parse(&naga::front::glsl::Options::from(naga::ShaderStage::Compute), &src)
            .unwrap_or_else(|e| panic!("Shader compile failed ({name}):\n{}", e.emit_to_string(&src)));
        let inputs = parse_inputs(&src);
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
        });
        let vis = wgpu::ShaderStages::COMPUTE;
        let mut entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: vis,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: FMT,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ];
        for b in 1..=2 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: b,
                visibility: vis,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }
        for b in 3..=6 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: b,
                visibility: vis,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        for (b, is3) in &inputs {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: *b,
                visibility: vis,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if *is3 { wgpu::TextureViewDimension::D3 } else { wgpu::TextureViewDimension::D2 },
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some(name), entries: &entries });
        let pl = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(name),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let k = Arc::new(Kernel { pipeline, layout, inputs });
        self.kernels.lock().unwrap().insert(name.to_string(), k.clone());
        k
    }

    pub fn dummy_tex(&self) -> T { self.dummy2d.clone() }

    /// Compiles every kernel up front (call from a background thread at startup).
    pub fn warm_up(&self) {
        for n in kernel_names() {
            self.kernel(n);
        }
    }

    fn raw_texture(&self, w: u32, h: u32, d: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> T {
        let w = w.clamp(1, self.max_dim);
        let h = h.clamp(1, self.max_dim);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: d.max(1) },
            mip_level_count: 1,
            sample_count: 1,
            dimension: if d > 1 { wgpu::TextureDimension::D3 } else { wgpu::TextureDimension::D2 },
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Arc::new(Tex { texture, view, w, h, d: d.max(1), format })
    }

    /// A working texture (rgba16float). Recycles textures no one references any more.
    pub fn make_texture(&self, w: usize, h: usize) -> T {
        let (w, h) = ((w.max(1) as u32).min(self.max_dim), (h.max(1) as u32).min(self.max_dim));
        {
            let found = POOL.with(|p| p.borrow().iter().find(|t| t.w == w && t.h == h && Arc::strong_count(t) == 1).cloned());
            if let Some(t) = found {
                return t;
            }
        }
        let t = self.raw_texture(
            w,
            h,
            1,
            FMT,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        );
        POOL.with(|p| p.borrow_mut().push(t.clone()));
        t
    }

    /// Drops pooled textures nobody uses, keeping at most `keep_bytes` of idle ones.
    pub fn trim_pool(&self, keep_bytes: u64) {
        let mut kept = 0u64;
        POOL.with(|p| p.borrow_mut().retain(|t| {
            if Arc::strong_count(t) > 1 { return true; }
            let b = t.w as u64 * t.h as u64 * 8;
            if kept + b <= keep_bytes { kept += b; true } else { false }
        }));
    }

    fn upload_bytes(&self, w: u32, h: u32, d: u32, format: wgpu::TextureFormat, bpp: u32, data: &[u8]) -> T {
        let t = self.raw_texture(w, h, d, format, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * bpp), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: d },
        );
        t
    }

    /// Float rgba data (lookup tables, masks...). Uses rgba32float when it can be filtered.
    pub fn texture_from_rgba_f32(&self, data: &[f32], w: usize, h: usize, d: usize) -> T {
        if self.f32_filterable {
            self.upload_bytes(w as u32, h as u32, d as u32, wgpu::TextureFormat::Rgba32Float, 16, bytemuck::cast_slice(data))
        } else {
            let hs: Vec<half::f16> = data.iter().map(|v| half::f16::from_f32(*v)).collect();
            self.upload_bytes(w as u32, h as u32, d as u32, FMT, 8, bytemuck::cast_slice(&hs))
        }
    }

    pub fn lut1d(&self, values: &[[f32; 4]]) -> T {
        let flat: Vec<f32> = values.iter().flatten().copied().collect();
        self.texture_from_rgba_f32(&flat, values.len(), 1, 1)
    }

    /// 8-bit rgba (sRGB-encoded values, stored as unorm so shaders see them as is).
    pub fn texture_from_rgba8(&self, data: &[u8], w: usize, h: usize) -> T {
        self.upload_bytes(w as u32, h as u32, 1, wgpu::TextureFormat::Rgba8Unorm, 4, data)
    }
}

/// Per-render context: command recording and the helpers used by effects.
pub struct Ctx {
    encoder: Option<wgpu::CommandEncoder>,
    /// Render pixels per reference pixel (see Renderer::REFERENCE_SIZE).
    pub scale: f64,
    pub width: usize,
    pub height: usize,
    /// Full-resolution source of the document (cropped), at render size.
    pub source: T,
    pub is_export: bool,
    passes: usize,
}

impl Ctx {
    pub fn new(scale: f64, source: T, is_export: bool) -> Ctx {
        let (width, height) = (source.width(), source.height());
        Ctx { encoder: None, scale, width, height, source, is_export, passes: 0 }
    }

    pub fn gpu(&self) -> &'static Gpu { gpu() }

    /// Converts a size in reference pixels to render pixels.
    pub fn px(&self, v: f64) -> f64 { v * self.scale }

    pub fn new_tex(&self, w: usize, h: usize) -> T { gpu().make_texture(w, h) }
    pub fn new_tex_like(&self, t: &T) -> T { gpu().make_texture(t.width(), t.height()) }

    fn enc(&mut self) -> &mut wgpu::CommandEncoder {
        if self.encoder.is_none() {
            self.encoder = Some(gpu().device.create_command_encoder(&Default::default()));
        }
        self.encoder.as_mut().unwrap()
    }

    /// Runs a compute kernel writing `out`, inputs bound at texture(1...).
    pub fn run(&mut self, kernel: &str, out: &T, inputs: &[&T], u: &[f32], extra: Option<&[f32]>) {
        let g = gpu();
        let k = g.kernel(kernel);
        let mut uu: Vec<f32> = u.to_vec();
        if uu.len() < 64 { uu.resize(64, 0.0); }
        let mut ee: Vec<f32> = extra.map(|e| e.to_vec()).unwrap_or_default();
        if ee.len() < 4 { ee.resize(4, 0.0); }
        let ub = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&uu),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let eb = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&ee),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let mut entries = vec![
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&out.view) },
            wgpu::BindGroupEntry { binding: 1, resource: ub.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: eb.as_entire_binding() },
        ];
        for (i, s) in g.samplers.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry { binding: 3 + i as u32, resource: wgpu::BindingResource::Sampler(s) });
        }
        for (i, (b, is3)) in k.inputs.iter().enumerate() {
            let idx = (*b - 7) as usize;
            let _ = i;
            let t: &T = match inputs.get(idx) {
                Some(t) => t,
                None => if *is3 { &g.dummy3d } else { &g.dummy2d },
            };
            entries.push(wgpu::BindGroupEntry { binding: *b, resource: wgpu::BindingResource::TextureView(&t.view) });
        }
        let bg = g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &k.layout, entries: &entries });
        let enc = self.enc();
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            pass.set_pipeline(&k.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(out.w.div_ceil(16), out.h.div_ceil(16), 1);
        }
        self.passes += 1;
        // Keep command buffers a reasonable size on long stacks.
        if self.passes >= 400 { self.submit(); }
    }

    pub fn apply(&mut self, kernel: &str, inputs: &[&T], u: &[f32]) -> T {
        let (w, h) = inputs.first().map(|t| (t.width(), t.height())).unwrap_or((self.width, self.height));
        self.apply_sized(kernel, inputs, u, None, w, h)
    }

    pub fn apply_ex(&mut self, kernel: &str, inputs: &[&T], u: &[f32], extra: &[f32]) -> T {
        let (w, h) = inputs.first().map(|t| (t.width(), t.height())).unwrap_or((self.width, self.height));
        self.apply_sized(kernel, inputs, u, Some(extra), w, h)
    }

    pub fn apply_sized(&mut self, kernel: &str, inputs: &[&T], u: &[f32], extra: Option<&[f32]>, w: usize, h: usize) -> T {
        let out = self.new_tex(w, h);
        self.run(kernel, &out, inputs, u, extra);
        out
    }

    pub fn blur(&mut self, src: &T, sigma: f64) -> T {
        let s = sigma.max(0.01);
        if s < 0.3 { return src.clone(); }
        // Large radii: blur a downscaled copy (fast + same look).
        if s > 12.0 {
            let f = ((s / 6.0) as usize).max(1);
            let small = self.resize(src, (src.width() / f).max(1), (src.height() / f).max(1), false);
            let b = self.blur(&small, s / f as f64);
            return self.resize(&b, src.width(), src.height(), true);
        }
        let h = self.apply("k_gauss", &[src], &[s as f32, 1.0, 0.0]);
        self.apply("k_gauss", &[&h], &[s as f32, 0.0, 1.0])
    }

    /// High-quality resize (Lanczos for downscale, bilinear for upscale / smooth).
    pub fn resize(&mut self, src: &T, w: usize, h: usize, smooth: bool) -> T {
        let (w, h) = (w.max(1), h.max(1));
        if w == src.width() && h == src.height() { return src.clone(); }
        if (w < src.width() || h < src.height()) && !smooth {
            let x = self.apply_sized("k_lanczos", &[src], &[0.0], None, w, src.height());
            let y = self.apply_sized("k_lanczos", &[&x], &[1.0], None, w, h);
            // Lanczos can ring below 0 / above 1.
            return self.apply("k_clamp", &[&y], &[]);
        }
        self.apply_sized("k_bilinear", &[src], &[], None, w, h)
    }

    pub fn upscale_nearest(&mut self, src: &T, w: usize, h: usize) -> T {
        self.apply_sized("k_nearest", &[src], &[], None, w, h)
    }

    pub fn upscale_blocks(&mut self, src: &T, pix: usize, w: usize, h: usize) -> T {
        self.apply_sized("k_block", &[src], &[pix as f32], None, w, h)
    }

    fn submit(&mut self) {
        if let Some(e) = self.encoder.take() {
            gpu().queue.submit([e.finish()]);
        }
        self.passes = 0;
    }

    /// Submits pending GPU work and waits.
    pub fn flush(&mut self) {
        self.submit();
        let _ = gpu().device.poll(wgpu::PollType::wait_indefinitely());
    }

    /// Reads a texture back as straight rgba f32.
    pub fn read_floats(&mut self, tex: &T) -> Vec<f32> {
        let src = if tex.format == FMT && tex.texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
            tex.clone()
        } else {
            self.apply("k_copy", &[tex], &[])
        };
        self.submit();
        let (w, h) = (src.w, src.h);
        let row = (w * 8).div_ceil(256) * 256;
        let g = gpu();
        let buf = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: row as u64 * h as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = g.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &src.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        g.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = g.device.poll(wgpu::PollType::wait_indefinitely());
        let data = slice.get_mapped_range().expect("map");
        let mut out = vec![0f32; (w * h * 4) as usize];
        for y in 0..h as usize {
            let r: &[half::f16] = bytemuck::cast_slice(&data[y * row as usize..y * row as usize + w as usize * 8]);
            for (i, v) in r.iter().enumerate() {
                out[y * w as usize * 4 + i] = v.to_f32();
            }
        }
        drop(data);
        buf.unmap();
        out
    }

    /// Uploads straight rgba f32 pixels as a working texture.
    pub fn upload(&mut self, floats: &[f32], w: usize, h: usize) -> T {
        let hs: Vec<half::f16> = floats.iter().map(|v| half::f16::from_f32(*v)).collect();
        let t = self.new_tex(w, h);
        self.submit();
        gpu().queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&hs),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w as u32 * 8), rows_per_image: Some(h as u32) },
            wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        );
        t
    }
}

impl Drop for Ctx {
    fn drop(&mut self) {
        self.submit();
    }
}
