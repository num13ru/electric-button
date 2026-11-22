//! GPU render callback for the electric button lightning effect.
//!
//! Uses egui-wgpu `Callback::new_paint_callback` and `CallbackTrait`
//! to render the animated border via WGSL shader.
//!
//! Pipeline and bind group layout are created once in `prepare`.
//! Per-button uniform buffer, sampler, and bind group are created
//! in `prepare` using device/queue, stored in a shared Vec, and
//! read back in `paint`.

use core::num::NonZeroU64;

use bytemuck::{cast_slice, Pod, Zeroable};
use egui::PaintCallbackInfo;
use egui_wgpu::CallbackTrait;
use wgpu::{
    util::DeviceExt as _, BindGroup, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, ColorTargetState, CommandEncoder,
    Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline, SamplerDescriptor,
    TextureFormat,
};

use crate::shader::create_shader_module;

/// Per-button GPU resources stored in shared Vec.
pub(crate) struct ButtonResources {
    /// Holds the uniform buffer alive across frames.
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
    /// Per-button resources indexed by callback counter.
    pub(crate) buttons: Vec<ButtonResources>,
}

/// Per-instance uniform data. Must be 16-byte aligned.
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ElectricButtonUniforms {
    /// Button rect in pixels (min_x, min_y, max_x, max_y)
    pub button_rect: [f32; 4],
    /// Screen size in pixels (width, height)
    pub screen_size: [f32; 2],
    /// Corner radius in pixels
    pub radius: f32,
    /// Border thickness in pixels
    pub thickness: f32,
    /// Border color (RGBA)
    pub color: [f32; 4],
    /// Glow spread in pixels
    pub glow_spread: f32,
    /// Animation padding in points
    pub glow_padding: f32,
    /// Animation time
    pub time: f32,
    /// Active multiplier
    pub is_active: f32,
}

impl ElectricButtonUniforms {
    pub fn new(
        callback_rect: egui::Rect,
        glow_padding: f32,
        pixels_per_point: f32,
        radius: f32,
        thickness: f32,
        color: egui::Color32,
        glow_spread: f32,
        time: f32,
        active: bool,
    ) -> Self {
        // TODO(stale comment — nothing is expanded here): show_impl passes the
        // UNEXPANDED `response.rect` as `callback_rect`, so button_rect is the
        // body rect in physical pixels. The shader maps UV 0..1 to `screen_size`
        // (not to button_rect); the expanded region is only the paint callback/clip
        // rect, and clipping to it comes from the scissor rect in paint().
        // button_rect itself = body rect in physical pixels.
        Self {
            button_rect: [
                callback_rect.min.x * pixels_per_point,
                callback_rect.min.y * pixels_per_point,
                callback_rect.max.x * pixels_per_point,
                callback_rect.max.y * pixels_per_point,
            ],
            screen_size: [0.0, 0.0], // filled in prepare()
            radius,
            thickness,
            color: [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
                color.a() as f32 / 255.0,
            ],
            glow_spread,
            glow_padding,
            time,
            is_active: if active { 1.0 } else { 0.0 },
        }
    }
}

/// Button render callback. Creates GPU resources in `prepare`,
/// draws in `paint`.
pub struct ElectricButtonCallback {
    pub uniforms: ElectricButtonUniforms,
}

impl ElectricButtonCallback {
    pub fn new(uniforms: ElectricButtonUniforms) -> Self {
        Self { uniforms }
    }
}

impl CallbackTrait for ElectricButtonCallback {
    fn prepare(
        &self,
        device: &Device,
        queue: &Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources = callback_resources
            .entry::<ElectricButtonPipelineResources>()
            .or_insert_with(|| ElectricButtonPipelineResources {
                pipeline: None,
                bind_group_layout: None,
                buttons: Vec::new(),
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
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

        // Create per-button entry.
        let idx = resources.buttons.len();
        let mut uniforms_copy = self.uniforms;
        uniforms_copy.screen_size = [
            screen_descriptor.size_in_pixels[0] as f32,
            screen_descriptor.size_in_pixels[1] as f32,
        ];
        let uniform_array = [uniforms_copy];
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
        // TODO(dead resource — sampler never sampled): the shader never samples
        // binding 1, so this sampler and its bind group entry are per-button waste.
        // Suggested fix: remove the sampler creation, the bind group layout entry,
        // and the `var s: sampler` binding in electric.wgsl (see TODO there).
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("electric_button_bind_group"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        resources.buttons.push(ButtonResources {
            uniform_buffer,
            bind_group,
        });

        // Store index for paint to use.
        callback_resources.insert::<u32>(idx as u32);

        Vec::new()
    }

    fn paint(
        &self,
        info: PaintCallbackInfo,
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

        let idx = match callback_resources.get::<u32>() {
            Some(&i) => i as usize,
            None => return,
        };
        let btn = match resources.buttons.get(idx) {
            Some(b) => b,
            None => return,
        };

        // TODO(bug — scissor rect can exceed the render target): min is clamped to
        // 0, but width/height are not clamped, so a button near the bottom/right
        // edge pushes the scissor past the render target and wgpu-core rejects it
        // (InvalidScissorRect). Also asymmetric: min is clamped at 0 but the grown
        // width/height are not clamped against the render target.
        // Suggested fix: clamp min to 0 and width/height to the render target
        // bounds (or skip the scissor entirely), using only `info.clip_rect`.
        let margin = 2i32;
        render_pass.set_scissor_rect(
            (info.clip_rect.min.x as i32).saturating_sub(margin).max(0) as u32,
            (info.clip_rect.min.y as i32).saturating_sub(margin).max(0) as u32,
            (info.clip_rect.width() as i32 + margin * 2).max(1) as u32,
            (info.clip_rect.height() as i32 + margin * 2).max(1) as u32,
        );

        // Draw.
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &btn.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
