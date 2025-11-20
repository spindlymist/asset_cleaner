use std::{fs, io};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use image::{EncodableLayout, imageops};

const IDLE_FPS: f64 = 60.0;
const IDLE_FRAME_DURATION_MS: f64 = 1000.0 / IDLE_FPS;

pub struct Extras<'a> {
    pub window: &'a sdl2::video::Window,
    pub fonts: &'a Fonts,
    pub textures: &'a mut Textures<'a>,
}

pub fn run<F>(imgui: ImguiState, mut build: F) where
    F: FnMut(&imgui::Ui, Extras)
{
    use sdl2::event::{Event, WindowEvent};
    
    let ImguiState { mut platform, mut renderer, context: mut im_context, fonts } = imgui;
    let event_pump = &mut platform.event_pump;
    let window = &mut platform.window;
    let im_platform = platform.im_platform.as_mut().expect("Platform should be initialized");
    let im_renderer = renderer.im_renderer.as_mut().expect("Renderer should be initialized");
    
    let mut last_frame_start = Instant::now();
    let mut is_in_background = false;
    
    // ui.push_style_color() doesn't work
    im_context.style_mut().colors[imgui::StyleColor::ModalWindowDimBg as usize] = [0.0, 0.0, 0.0, 0.95];
    
    'main_loop: loop {
        // Calculate how long to wait for the next event without missing fps target
        let last_frame_duration_ms = (Instant::now() - last_frame_start).as_secs_f64() * 1000.0;
        let wait_ms = f64::max(0.0, f64::round(IDLE_FRAME_DURATION_MS - last_frame_duration_ms)) as u32;
        let first_event = event_pump.wait_event_timeout(wait_ms);
        last_frame_start = Instant::now();
        
        // Process events
        if let Some(first_event) = first_event {
            let events = [first_event]
                .into_iter()
                .chain(event_pump.poll_iter());
            for event in events {
                im_platform.handle_event(&mut im_context, &event);
                
                if let Event::Window { window_id, .. } = event
                    && window_id != window.id()
                {
                    continue;
                }
                
                match event {
                    Event::Window { win_event: WindowEvent::SizeChanged(width, height), .. } => {
                        renderer.surface_config.width = width as u32;
                        renderer.surface_config.height = height as u32;
                        renderer.surface.configure(&renderer.device, &renderer.surface_config);
                    }
                    Event::Window { win_event: WindowEvent::Minimized, .. } => {
                        is_in_background = true;
                    }
                    Event::Window { win_event: WindowEvent::Exposed, .. } => {
                        is_in_background = false;
                        renderer.surface.configure(&renderer.device, &renderer.surface_config);
                    }
                    Event::Quit { .. } => break 'main_loop,
                    _ => {},
                }
            }
        }
        
        if is_in_background {
            continue;
        }
        
        // Prepare for new frame
        im_platform.prepare_frame(&mut im_context, window, event_pump);
        let ui = im_context.new_frame();
        
        // Build UI
        {
            let mut textures = Textures {
                renderer: im_renderer,
                device: &renderer.device,
                queue: &renderer.queue,
            };
            let extras = Extras {
                window,
                fonts: &fonts,
                textures: &mut textures,
            };
            build(ui, extras);
        }
        
        // Prepare to render
        let draw_data = im_context.render();
        let frame = match renderer.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                eprintln!("Failed to get current surface texture: {err}");
                continue 'main_loop;
            }
        };
        let render_target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command encoder"),
        });

        // Render imgui
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            im_renderer.render(draw_data, &renderer.queue, &renderer.device, &mut render_pass)
                .expect("Failed to render");
        }

        // Submit commands and present next frame
        renderer.queue.submit([encoder.finish()]);
        frame.present();
    }
}

#[allow(dead_code)]
pub struct PlatformState {
    pub sdl: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
    pub event_pump: sdl2::EventPump,
    pub im_platform: Option<imgui_sdl2_support::SdlPlatform>,
}

pub fn platform_init(title: &str, size: (u32, u32)) -> Result<PlatformState> {
    let sdl = match sdl2::init() {
        Ok(value) => value,
        Err(err) => anyhow::bail!(err)
    };
    let video_subsystem = match sdl.video() {
        Ok(value) => value,
        Err(err) => anyhow::bail!(err),
    };
    let window = video_subsystem.window(title, size.0, size.1)
        .position_centered()
        .resizable()
        .metal_view()
        .build()?;
    let event_pump = match sdl.event_pump() {
        Ok(value) => value,
        Err(err) => anyhow::bail!(err),
    };
    sdl2::hint::set("SDL_MOUSE_FOCUS_CLICKTHROUGH", "1");
    
    Ok(PlatformState {
        sdl,
        video_subsystem,
        window,
        event_pump,
        im_platform: None,
    })
}

pub struct RendererState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub im_renderer: Option<imgui_wgpu::Renderer>,
}

pub fn renderer_init<W>(window: &W, size: (u32, u32)) -> Result<RendererState>
where
    W: raw_window_handle::HasDisplayHandle + raw_window_handle::HasWindowHandle
{
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window).unwrap())?
    };
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))?;
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("device"),
            required_limits: wgpu::Limits::default(),
            required_features: wgpu::Features::empty(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        },
    ))?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.0,
        height: size.1,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: Vec::default(),
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    Ok(RendererState {
        device,
        queue,
        surface,
        surface_config,
        im_renderer: None,
    })
}

pub struct ImguiState {
    pub platform: PlatformState,
    pub renderer: RendererState,
    pub context: imgui::Context,
    pub fonts: Fonts,
}

pub struct Fonts {
    pub ui: imgui::FontId,
    pub mono: imgui::FontId,
}

pub fn imgui_init(mut platform: PlatformState, mut renderer: RendererState) -> ImguiState {
    let mut context = imgui::Context::create();
    context.set_ini_filename(None);
    context.set_log_filename(None);
    
    let ui_font = context.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../resources/FiraSans-Regular.ttf"),
            size_pixels: 19.0,
            config: None,
        }
    ]);
    
    let mono_font = context.fonts().add_font(&[
        imgui::FontSource::TtfData {
            data: include_bytes!("../resources/FiraCode-Regular.ttf"),
            size_pixels: 19.0,
            config: None,
        }
    ]);
    
    let fonts = Fonts {
        ui: ui_font,
        mono: mono_font,
    };
    
    let imgui_platform = imgui_sdl2_support::SdlPlatform::new(&mut context);
    platform.im_platform = Some(imgui_platform);

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: renderer.surface_config.format,
        ..Default::default()
    };
    let imgui_renderer = imgui_wgpu::Renderer::new(&mut context, &renderer.device, &renderer.queue, renderer_config);
    renderer.im_renderer = Some(imgui_renderer);

    ImguiState {
        context,
        platform,
        renderer,
        fonts,
    }
}

pub struct Textures<'a> {
    renderer: &'a mut imgui_wgpu::Renderer,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

pub struct Image {
    pub texture_id: imgui::TextureId,
    pub width: f32,
    pub height: f32,
    pub is_downscaled: bool,
}

impl<'a> Textures<'a> {
    pub fn create_texture_from_file(&mut self, path: &Path) -> Result<Image> {
        let max_size = self.device.limits().max_texture_dimension_2d;
        let (image, is_downscaled) = load_and_scale_image(path, max_size)?;
        let width = image.width();
        let height = image.height();
        let raw_bytes = image.as_bytes();
        
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(self.device, self.renderer, texture_config);
        texture.write(self.queue, raw_bytes, width, height);
        let texture_id = self.renderer.textures.insert(texture);
        
        Ok(Image {
            texture_id,
            width: width as f32,
            height: height as f32,
            is_downscaled,
        })
    }
    
    pub fn delete_texture(&mut self, texture_id: imgui::TextureId) {
        // imgui_wgpu::Texture has a pointer to the underlying wgpu::Texture.
        // When wgpu::Texture is dropped, the texture is automatically destroyed.
        self.renderer.textures.remove(texture_id);
    }
}

pub fn load_and_scale_image(path: &Path, max_size: u32) -> Result<(image::RgbaImage, bool)> {
    let decoder = {
        let file = match fs::OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            // Err(err) if err.kind() == io::ErrorKind::NotFound => {
            //     return Ok(None);
            // },
            Err(err) => Err(err)?,
        };
        crate::png_decoder::PngDecoder::new(io::BufReader::new(file))
            .with_context(|| format!("Error while decoding image {path:?}"))?
    };

    let image_dynamic = image::DynamicImage::from_decoder(decoder)
        .with_context(|| format!("Error while decoding image {path:?}"))?;
    let image_rgba =
        if let image::DynamicImage::ImageRgba8(image_rgba8) = image_dynamic {
            image_rgba8
        }
        else {
            image_dynamic.to_rgba8()
        };
    let width = image_rgba.width();
    let height = image_rgba.height();
    
    if width <= max_size && height <= max_size {
        Ok((image_rgba, false))
    }
    else {
        let (new_width, new_height) =
            if width >= height {
                let ratio = height as f32 / width as f32;
                (
                    max_size,
                    f32::round(ratio * max_size as f32) as u32
                )
            }
            else {
                let ratio = width as f32 / height as f32;
                (
                    f32::round(ratio * max_size as f32) as u32,
                    max_size
                )
            };
        let image_scaled = imageops::resize(&image_rgba, new_width, new_height, imageops::FilterType::CatmullRom);

        Ok((image_scaled, true))
    }
}
