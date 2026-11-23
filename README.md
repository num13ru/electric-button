# electric-button

A clickable button widget with a GPU-rendered lightning border, built with
egui/eframe and wgpu.

## Features

- GPU-rendered electric-border animation (WGSL shader via `wgpu::include_wgsl!`)
- Multiple interaction states: Idle, Hovered, Pressed, Focused, Disabled
- Glow spread, corner radius and border thickness are configurable per button
- Per-frame uniform upload; pipeline and bind group layout are shared per device

## Requirements

- Rust (edition 2024)
- A working wgpu-capable GPU (native window on macOS/Windows/Linux)

## Build and run

```sh
cargo run
```

This launches the demo app (`src/app.rs`) showing multiple `ElectricButton`
instances with GPU-rendered lightning animation.

## Usage

```rust
use electric_button::electric_button::ElectricButton;
use electric_button::electric_button_style::ElectricButtonStyle;

let button = ElectricButton::new("Click me")
    .desired_size(egui::Vec2::new(200.0, 64.0))
    .style(ElectricButtonStyle::default());

let (response, rect, state, _style) = button.show(ui, global_time);
if response.clicked() {
    // handle click
}
```

See `src/app.rs` for a full demo.

## Project layout

- `src/app.rs` — demo application
- `src/electric_button.rs` — widget definition
- `src/electric_button_renderer.rs` — GPU render callback (wgpu)
- `src/electric_button_state.rs` — interaction states
- `src/electric_button_style.rs` — visual parameters
- `src/shader.rs` + `src/shaders/electric.wgsl` — WGSL shader
- `doc/` — design plan

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build
```
