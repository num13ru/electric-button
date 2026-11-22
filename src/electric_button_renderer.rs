//! GPU render callback for the electric button lightning effect.
//!
//! Uses egui-wgpu `Callback::new_paint_callback` and `CallbackTrait`
//! to render the animated border via WGSL shader.
//!
//! Pipeline and bind group layout are created once in `prepare` (shared
//! via `CallbackResources`). Per-callback uniform buffer and bind group are
//! created in `prepare` and kept in the callback itself, so each button paints
//! its own bind group rather than sharing one slot in a common map.

use core::num::NonZeroU64;

use bytemuck::{cast_slice, Pod, Zeroable};
use egui::PaintCallbackInfo;
use egui_wgpu::CallbackTrait;
use wgpu::{
    util::DeviceExt as _, BindGroup, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, ColorTargetState, CommandEncoder,
    Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    TextureFormat,
};

use crate::shader::create_shader_module;

/// Per-button GPU resources for one frame. Lives inside the callback so each
/// button owns its own buffer and bind group.
pub(crate) struct ButtonResources {
    /// Keeps the uniform buffer alive while the bind group is in use.
    // TODO(dead-code): intentionally unread — holding the buffer reference
    // keeps the GPU resource alive for the lifetime of the bind group.
    #[allow(dead_code)]
    uniform_buffer: Buffer,
    bind_group: BindGroup,
}

/// Shared GPU resources created once per device.
pub(crate) struct ElectricButtonPipelineResources {
    /// Reusable render pipeline.
    pub(crate) pipeline: Option<RenderPipeline>,
    /// Reusable bind group layout.
    pub(crate) bind_group_layout: Option<BindGroupLayout>,
}

/// Per-instance uniform data. Layout must match the WGSL `Uniforms`
/// struct (struct alignment 16, size 80), so it carries explicit padding
/// fields instead of implicit padding (bytemuck requires no padding).
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ElectricButtonUniforms {
    /// Body rect in physical pixels (min_x, min_y, max_x, max_y)
    pub button_rect: [f32; 4],
    /// Expanded callback rect in physical pixels (min_x, min_y, max_x, max_y)
    pub viewport: [f32; 4],
    /// Corner radius in pixels
    pub radius: f32,
    /// Border thickness in pixels
    pub thickness: f32,
    /// Padding so `color` lands on a 16-byte boundary (WGSL vec4 alignment).
    pub _pad0: [f32; 2],
    /// Border color (RGBA)
    pub color: [f32; 4],
    /// Glow spread in pixels
    pub glow_spread: f32,
    /// Animation time
    pub time: f32,
    /// Active multiplier
    pub is_active: f32,
    /// Trailing padding so the struct size (80) matches the WGSL struct.
    pub _pad: f32,
}

impl ElectricButtonUniforms {
    pub fn new(
        body_rect: egui::Rect,
        viewport_rect: egui::Rect,
        pixels_per_point: f32,
        radius: f32,
        thickness: f32,
        color: egui::Color32,
        glow_spread: f32,
        time: f32,
        active: bool,
    ) -> Self {
        Self {
            button_rect: [
                body_rect.min.x * pixels_per_point,
                body_rect.min.y * pixels_per_point,
                body_rect.max.x * pixels_per_point,
                body_rect.max.y * pixels_per_point,
            ],
            viewport: [
                viewport_rect.min.x * pixels_per_point,
                viewport_rect.min.y * pixels_per_point,
                viewport_rect.max.x * pixels_per_point,
                viewport_rect.max.y * pixels_per_point,
            ],
            radius: radius * pixels_per_point,
            thickness: thickness * pixels_per_point,
            color: [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
                color.a() as f32 / 255.0,
            ],
            glow_spread: glow_spread * pixels_per_point,
            time,
            is_active: if active { 1.0 } else { 0.0 },
            _pad0: [0.0, 0.0],
            _pad: 0.0,
        }
    }
}

/// Button render callback. Creates GPU resources in `prepare`,
/// draws in `paint`.
pub struct ElectricButtonCallback {
    pub uniforms: ElectricButtonUniforms,
    resource: std::sync::Mutex<Option<ButtonResources>>,
}

impl ElectricButtonCallback {
    pub fn new(uniforms: ElectricButtonUniforms) -> Self {
        Self {
            uniforms,
            resource: std::sync::Mutex::new(None),
        }
    }
}

impl CallbackTrait for ElectricButtonCallback {
    fn prepare(
        &self,
        device: &Device,
        queue: &Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources = callback_resources
            .entry::<ElectricButtonPipelineResources>()
            .or_insert_with(|| ElectricButtonPipelineResources {
                pipeline: None,
                bind_group_layout: None,
            });

        // Create bind group layout once.
        if resources.bind_group_layout.is_none() {
            let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("electric_button_bg_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(core::mem::size_of::<ElectricButtonUniforms>() as u64),
                        },
                        count: None,
                    },
                ],
            });
            resources.bind_group_layout = Some(bgl);
        }

        // Create pipeline once.
        if resources.pipeline.is_none() {
            let shader = create_shader_module(device);

            let bgl = resources.bind_group_layout.as_ref().unwrap();
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("electric_button_pipeline_layout"),
                bind_group_layouts: &[Some(bgl)],
                immediate_size: 0,
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("electric_button_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Bgra8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    polygon_mode: PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

            resources.pipeline = Some(pipeline);
        }

        // Create this callback's uniform buffer and bind group.
        let uniform_array = [self.uniforms];
        let uniform_bytes = cast_slice(&uniform_array);
        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("electric_button_uniform"),
                contents: uniform_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );
        queue.write_buffer(&uniform_buffer, 0, uniform_bytes);

        let bgl = resources.bind_group_layout.as_ref().unwrap();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("electric_button_bind_group"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        *self.resource.lock().unwrap() = Some(ButtonResources {
            uniform_buffer,
            bind_group,
        });

        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let resources = match callback_resources.get::<ElectricButtonPipelineResources>() {
            Some(r) => r,
            None => return,
        };
        let pipeline = match &resources.pipeline {
            Some(p) => p,
            None => return,
        };
        let resource_guard = self.resource.lock().unwrap();
        let btn = match resource_guard.as_ref() {
            Some(b) => b,
            None => return,
        };

        // Draw with the viewport that egui_wgpu set to the (expanded)
        // callback rect — no manual viewport or scissor manipulation.
        // Draw.
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &btn.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
