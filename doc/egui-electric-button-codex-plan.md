# Plan: Animated Electric-Border Button in egui (wgpu/WGSL)

## Goal

Build a reusable `ElectricButton` widget for a Rust desktop application using `egui`, with a procedural, animated lightning effect following a rounded-rectangle outline. The button must preserve native `egui` layout, interaction, accessibility where supported, and normal text rendering. Render the effect on the GPU using WGSL via `egui-wgpu`.

## Assumptions and repository constraints

- Native desktop is the initial target; verify on macOS/Metal. Keep Windows/Linux portability in mind without claiming untested platform support.
- Prefer the `wgpu` renderer through `eframe`. Use versions compatible with the repository's Rust toolchain and lockfile; do not force an unrelated major upgrade.
- At the time of writing, the published `egui-wgpu` API supports `Callback::new_paint_callback`, `CallbackTrait`, and renderer callback resources. Verify signatures against the **locked dependency version**, not `main` examples alone.
- The effect is cosmetic. A missing/unsupported GPU pipeline must not silently disable pointer interaction or crash the application: a plain `egui` button is an acceptable fallback.

## Non-goals

- No full post-processing engine or multipass bloom system in the first version.
- No custom event loop, retained-mode UI framework, shader hot-reloading, particle engine, texture assets, or external shader libraries.
- No requirement to perfectly imitate a specific copyrighted game's visual identity.

## Target behavior

The demo displays a visually clear, dark-themed button with:

1. A static rounded-rectangle base and readable label.
2. Thin electric-blue/cyan lightning traveling around **the perimeter**, not over the text.
3. An intense narrow core, irregular branching micro-arcs, and soft outer glow.
4. Smooth transitions among `idle`, `hovered`, `pressed`, `focused`, and `disabled` states.
5. A visible click result in the demo (e.g., a counter) to prove that actual `egui` interaction still works.
6. Configurable size, corner radius, color, border thickness, intensity, animation speed, and optional reduced-motion behavior.

The visual effect must not change the input hitbox. It must clip correctly with nearby widgets and respect UI scaling.

## Architecture

Separate responsibilities clearly:

- **`ElectricButton` (egui widget):** reserves layout, computes the response using standard `egui` interaction, defines the button state, draws or arranges the label, and requests repaints only when animation is needed.
- **`ElectricButtonStyle`:** describes visual parameters and state-specific intensity. Keep it public or otherwise reusable rather than hard-coding the demo theme into the shader.
- **`ElectricButtonRenderer` (egui-wgpu integration):** owns reusable GPU pipeline and draw resources. It receives geometry, time, visual parameters, and state from the widget; it does not manage clicks.
- **WGSL shader:** calculates the rounded-rectangle signed distance field (SDF), creates time-varying contour irregularity and short sparks, and composites core, border, and glow. Keep effects within a deliberately padded drawing region.
- **Demo app:** configures appearance, exposes a few controls for tuning, shows several button states, and displays activation feedback.

Use `egui_wgpu::Callback::new_paint_callback` and a `CallbackTrait` implementation, or an equivalent API supported by the locked version. Initialize pipelines once per device/format and reuse them. Use `CallbackResources` or the render-state-supported resource mechanism. Do not compile WGSL, recreate pipelines, or allocate significant GPU resources on every repaint.

### Drawing and clipping contract

- Distinguish three rectangles: **interaction rect** (clickable), **visual body rect** (rounded button), and **effect rect** (body plus glow padding).
- Ensure the effect rect is available to the painter/callback without expanding the click target. Reserve sufficient layout spacing or constrain glow to a known region to avoid clipping and overlap.
- Draw the GPU effect behind the label; draw text through `egui` for correct font scaling, accessibility support, and sharpness.
- Respect the parent clip rectangle and scissor applied by `egui-wgpu`.
- Handle pixels-per-point, window resizing, different aspect ratios, and fractional coordinates consistently.
- Use an appropriate alpha-blend setup for the target framebuffer. Avoid dark halos and assuming that naive additive blending matches `egui`'s existing render-pass state.

## Implementation milestones

### Phase 0 — Inspect the project and lock the approach

- [x] Determine whether an egui/eframe application already exists in the checked-out branch.
- [x] Pick the smallest integration location and document the reason in the PR/implementation notes.
- [x] Confirm the chosen `eframe`, `egui`, `egui-wgpu`, and `wgpu` versions are compatible.
- [x] Review the upstream `custom3d_wgpu` demo and current `CallbackTrait` documentation before coding.
- [x] Write a short implementation outline naming modules and ownership boundaries.

### Phase 1 — Runnable baseline and interaction

- [ ] Create or extend one desktop demo using `eframe` with its `wgpu` backend.
- [ ] Build `ElectricButton` initially as a plain clickable widget with customizable geometry and label.
- [ ] Use `egui` to manage hover, press, focus, disabled state, and actual activation.
- [ ] Add at least one activation counter and show the state in the demo.
- [ ] Confirm multiple buttons have independent state and that keyboard activation works where supported by the chosen `egui` API.

**Exit condition:** correct, testable button behavior without any shader.

### Phase 2 — GPU callback and static outline

- [ ] Add a minimal render callback that draws only inside its assigned region.
- [ ] Compile a WGSL shader with a rounded-rectangle SDF; first draw an unanimated border.
- [ ] Supply actual button geometry, resolution/scaling, radius, border thickness, and color as GPU parameters.
- [ ] Check that text remains sharp and visually above the shader effect.
- [ ] Verify two differently sized buttons render correctly in the same frame.
- [ ] Inspect resizing, DPI scaling, rounded corners, alpha blending, and parent clipping.

**Exit condition:** an accurately aligned, scalable, static GPU-rendered border.

### Phase 3 — Procedural lightning animation

- [ ] Animate one or more bright energy packets traveling along the perimeter using continuous time.
- [ ] Modulate border displacement/brightness with deterministic procedural noise; avoid frame-to-frame random flicker unrelated to time.
- [ ] Add a sharply defined bright core and a bounded soft-glow falloff within the same pass.
- [ ] Add sparse, short branching sparks anchored near the contour; keep them outside the text area.
- [ ] Ensure motion progresses smoothly around all four corners and does not visibly jump at the perimeter wraparound.
- [ ] Expose a deterministic time override for visual tests/screenshot reproducibility.
- [ ] Keep shader loops bounded and avoid excessive noise sampling or full-window offscreen textures.

**Exit condition:** animated lightning visibly follows the border, including the curved corners, without obscuring the label.

### Phase 4 — Interaction-aware animation and controls

- [ ] `idle`: faint stable outline, optional low-duty-cycle pulse.
- [ ] `hovered`: increased arc activity, speed, and brightness.
- [ ] `pressed`: brief flash and higher intensity without altering click semantics.
- [ ] `focused`: unmistakable focus indication that does not depend exclusively on color or animation.
- [ ] `disabled`: subdued, static visuals and no activation.
- [ ] Support reduced-motion mode: freeze or simplify moving effects while keeping focus/hover visible.
- [ ] Schedule repaints only during active animation; use `request_repaint_after` when appropriate and avoid a perpetual 60 FPS loop for inactive static buttons.
- [ ] Add demo controls for speed, intensity, radius, and color; keep defaults sensible.

**Exit condition:** interaction is reliable, animations are responsive, and inactive windows do not perform unnecessary rendering.

### Phase 5 — Reliability, fallback, and verification

- [ ] Do not recreate shader modules, pipelines, or heavyweight GPU resources per button per frame.
- [ ] Correctly handle multiple instances with distinct GPU parameters; prevent one button's uniforms from overwriting another's in the same render pass.
- [ ] Handle initialization errors and absent/unsupported GPU rendering explicitly. Prefer a functional plain-button fallback where feasible.
- [ ] Check the behavior with small buttons, large buttons, unusual aspect ratios, rapid repeated clicks, window resize, and DPI changes.
- [ ] Verify draw order, clipping, disabled hit testing, keyboard/focus behavior, and reduced motion.
- [ ] Add focused unit tests for pure geometry/state parameter mapping. Add deterministic visual screenshots or manual golden-frame checks if the environment supports them.
- [ ] Benchmark the demo with 1, 10, and 50 buttons. Report frame time and idle CPU/GPU behavior; do not assert a performance target without measurement.
- [ ] Run `cargo fmt --all -- --check`, relevant `cargo check`/`cargo test`/`cargo clippy` commands, and `git diff --check`. Explain any checks blocked by platform/hardware.

**Exit condition:** verified interaction, stable visuals, clean shutdown, and no regression to existing tools.

### Phase 6 — Document and package

- [ ] Add a brief README section or demo-specific `README.md` with run command and a screenshot/GIF **only if generated and verified**.
- [ ] Document shader inputs, how to reuse `ElectricButton`, supported renderer/backend assumptions, and limitations.
- [ ] Summarize changed files, completed phases, verification commands and results, unsupported platforms, and known tradeoffs.

## Acceptance criteria

- [ ] `ElectricButton` is reusable without copying shader code into each application screen.
- [ ] On the verified desktop target, the electric effect visibly follows the rounded border and remains animated during hover.
- [ ] No flashes, branches, or glow visually cover the button label.
- [ ] Clicking fires exactly one action per valid activation; dragging off cancels if that is the chosen standard `egui` behavior.
- [ ] Keyboard focus/activation and disabled behavior follow standard `egui` conventions where available.
- [ ] Multiple buttons animate independently, including different sizes and settings.
- [ ] Parent clipping, high-DPI rendering, window resizing, and blend behavior have been inspected.
- [ ] There is no shader compilation or full pipeline recreation in the render loop.
- [ ] Reduced-motion and fallback behavior are clear and documented.
- [ ] The demo starts from a documented command and required checks pass (or blockers are reported explicitly).

## Risks / design decisions to record

1. **Glow clipping:** egui callbacks are clipped; the effect region needs explicit padding, without increasing click area.
2. **GPU backend:** a WGSL callback depends on `wgpu`; it cannot automatically run under a Glow/OpenGL renderer.
3. **Transparency/composition:** a luminous border requires correct blending and gamma handling; a one-pass glow is an approximation, not full-screen bloom.
4. **Render-state lifetime:** per-instance uniforms and shared renderer resources must be safe across frame updates and multiple widget instances.
5. **Performance/power:** continuous idle animation wakes the render loop; prefer active-state rendering and reduced-motion support.
6. **Portability:** macOS/Metal is the initial validation target; Windows/Linux claims require testing.

## Possible follow-ups (out of scope for the first implementation)

- 💡 Animated energy that accelerates toward the pointer's nearest border position.
- 💡 A click-triggered shockwave that runs around the contour and naturally decays.
- 💡 Theme presets for cyan, amber, violet, and red electrical states.
- 💡 A hover-only effect mode to minimize GPU work and power consumption.
- 💡 An optional multipass bloom renderer, measured against the simpler single-pass glow.

## Codex execution instructions

Implement in the order above. Complete the smallest runnable milestone before adding the next one. Prefer straightforward modules and repository conventions over a generalized rendering framework. Verify with real build/tests and, where possible, an actual desktop run. Do not claim that the visuals or performance were verified solely because the code compiled. Record any constraints or tests you could not verify.

## References

- egui-wgpu callback API: https://docs.rs/egui-wgpu/latest/egui_wgpu/trait.CallbackTrait.html
- egui-wgpu renderer source: https://docs.rs/egui-wgpu/latest/src/egui_wgpu/renderer.rs.html
- Upstream custom wgpu rendering example: https://github.com/emilk/egui/blob/main/crates/egui_demo_app/src/apps/custom3d_wgpu.rs
- eframe: https://docs.rs/eframe/latest/eframe/
