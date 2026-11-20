//! Shader module for the electric button lightning effect.
//!
//! The WGSL source lives in `shaders/electric.wgsl` and is compiled
//! at build time via `wgpu::include_wgsl!`.

pub const ELECTRIC_SHADER_ID: &str = "electric";

/// Load the shader module from the embedded WGSL source.
pub fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::include_wgsl!("shaders/electric.wgsl"))
}
