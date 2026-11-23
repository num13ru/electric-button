//! GPU render callback for the electric button lightning effect.
//!
//! Uses egui-wgpu `Callback::new_paint_callback` and `CallbackTrait`
//! to render the animated border via WGSL shader.
//!
//! Pipeline and bind group layout are created once in `prepare` (shared
//! via `CallbackResources`). Per-callback uniform buffer and bind group are
//! created in `prepare` and kept in the callback itself, so each button paints
//! its own bind group rather than sharing one slot in a common map.
//! Uniform data is uploaded once via `create_buffer_init` (which writes the
//! contents); a separate `queue.write_buffer` with identical contents would
//! be redundant.

use core::num::NonZeroU64;

use bytemuck::{Pod, Zeroable, cast_slice};
use egui::PaintCallbackInfo;
use egui_wgpu::CallbackTrait;
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    Buffer, ColorTargetState, CommandEncoder, Device, FragmentState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass,
    RenderPipeline, TextureFormat, util::DeviceExt as _,
};

use crate::shader::create_shader_module;

/// Per-button GPU resources, shared across frames via an Arc: the buffer
/// and bind group are created once on first `prepare` and reused; each frame
/// only the uniform bytes are re-uploaded via `queue.write_buffer`.
pub(crate) struct ButtonResources {
    /// Keeps the uniform buffer alive while the bind group is in use;
    /// also used to re-upload uniform bytes each frame.
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
    /// State-dependent brightness multiplier (Idle 1.0, Hovered 1.3,
    /// Pressed 1.5, Focused 1.15, Disabled 0.0). Reuses the trailing
    /// padding slot so the struct size (80) and offsets are unchanged.
    pub intensity: f32,
}

impl ElectricButtonUniforms {
    // Grouping the constructor arguments would duplicate the struct's own
    // fields and add a second source of truth; the 10-positional-arg form is
    // used once per frame per callback during show_impl.
    #[allow(clippy::too_many_arguments)]
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
        intensity: f32,
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
            intensity,
        }
    }
}

/// Button render callback. Creates GPU resources in `prepare`,
/// draws in `paint`.
pub(crate) struct ElectricButtonCallback {
    uniforms: ElectricButtonUniforms,
    resource: std::sync::Arc<std::sync::Mutex<Option<ButtonResources>>>,
}

impl ElectricButtonCallback {
    pub(crate) fn new(
        uniforms: ElectricButtonUniforms,
        resource: std::sync::Arc<std::sync::Mutex<Option<ButtonResources>>>,
    ) -> Self {
        Self { uniforms, resource }
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
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            core::mem::size_of::<ElectricButtonUniforms>() as u64,
                        ),
                    },
                    count: None,
                }],
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

        // Reuse the per-callback GPU resources across frames; only create
        // them on first prepare. Later frames just re-upload the bytes.
        let bgl = resources.bind_group_layout.as_ref().unwrap();
        let mut guard = self.resource.lock().unwrap();
        match guard.as_mut() {
            Some(res) => {
                let uniform_array = [self.uniforms];
                let bytes = cast_slice(&uniform_array);
                queue.write_buffer(&res.uniform_buffer, 0, bytes);
            }
            None => {
                let uniform_array = [self.uniforms];
                let uniform_bytes = cast_slice(&uniform_array);
                let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("electric_button_uniform"),
                    contents: uniform_bytes,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("electric_button_bind_group"),
                    layout: bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }],
                });
                *guard = Some(ButtonResources {
                    uniform_buffer,
                    bind_group,
                });
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_struct_matches_wgsl_layout() {
        // The GPU reads exactly the bytes of this struct; WGSL declares the
        // same fields at the same offsets. If these change, the shader reads
        // garbage — this test pins the contract.
        assert_eq!(core::mem::size_of::<ElectricButtonUniforms>(), 80);
        assert_eq!(core::mem::offset_of!(ElectricButtonUniforms, is_active), 72);
        assert_eq!(core::mem::offset_of!(ElectricButtonUniforms, intensity), 76);
    }

    #[test]
    fn uniforms_carry_expected_layout() {
        use egui::Rect;
        let r = Rect::from_min_max(egui::Pos2::new(8.0, 12.0), egui::Pos2::new(128.0, 57.0));
        let u = ElectricButtonUniforms::new(
            r,
            r,
            2.0,
            8.0,
            2.0,
            egui::Color32::from_rgb(255, 165, 0),
            6.0,
            12.5,
            true,
            1.3,
        );
        assert_eq!(u.button_rect, [16.0, 24.0, 256.0, 114.0]);
        assert_eq!(u.radius, 16.0);
        assert_eq!(u.thickness, 4.0);
        assert_eq!(u.glow_spread, 12.0);
        assert_eq!(u.time, 12.5);
        assert_eq!(u.is_active, 1.0);
        assert_eq!(u.intensity, 1.3);
        assert!((u.color[0] - 1.0).abs() < 1e-6);
        assert!((u.color[1] - 165.0 / 255.0).abs() < 1e-3);
    }
}
