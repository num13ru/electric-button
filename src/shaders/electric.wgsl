// WGSL shader for procedural electric lightning border.
//
// Draws a rounded-rectangle SDF with time-varying procedural
// noise displacement for the lightning effect. Energy packets
// travel along the perimeter creating bright spots.

struct Uniforms {
    /// Button rect in pixels (min_x, min_y, max_x, max_y)
    button_rect: vec4<f32>,
    /// Screen size in pixels (width, height)
    screen_size: vec2<f32>,
    /// Corner radius in pixels
    radius: f32,
    /// Border thickness in pixels
    thickness: f32,
    /// Border color (RGBA)
    color: vec4<f32>,
    /// Glow spread in pixels
    glow_spread: f32,
    /// Animation padding in points
    glow_padding: f32,
    /// Animation time
    time: f32,
    /// Active multiplier
    is_active: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var s: sampler;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VSOut {
    let pos = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vertex_index] * 2.0 - 1.0, 0.0, 1.0);
    out.uv = pos[vertex_index];
    return out;
}

fn hash3(p: vec3<f32>) -> f32 {
    let s = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6)),
    );
    return fract(sin(s.x) * 43758.5453123);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;
    var i = 0u;
    while (i < 3u) {
        v += a * noise3d(pos);
        pos = pos * 2.0;
        a *= 0.5;
        i += 1u;
    }
    return v;
}

fn sdf_rounded_rect(p: vec2<f32>, rect: vec4<f32>, radius: f32) -> f32 {
    let d = vec2<f32>(
        max(rect.x - p.x, max(0.0, p.x - rect.z)),
        max(rect.y - p.y, max(0.0, p.y - rect.w)),
    );
    return length(d) - radius;
}

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    // Convert UV to pixel coords
    let pixel = input.uv * u.screen_size;

    // SDF
    let d = sdf_rounded_rect(pixel, u.button_rect, u.radius);

    // Time
    let t = u.time;
    let act = u.is_active;

    // Traveling energy packets along perimeter
    let tw = u.button_rect.z - u.button_rect.x;
    let th = u.button_rect.w - u.button_rect.y;
    let perimeter = 2.0 * tw + 2.0 * th;
    let num_packets = 3.0;
    var energy = vec3<f32>(0.0);
    var i = 0u;
    while (i < 3u) {
        let offset = t * 30.0 + f32(i) * (perimeter / num_packets);
        let edge = offset - (perimeter * (offset / perimeter));

        var ep: vec2<f32>;
        let top = vec2<f32>(u.button_rect.x, u.button_rect.y);
        let rtop = vec2<f32>(u.button_rect.z, u.button_rect.y);
        let rbot = vec2<f32>(u.button_rect.z, u.button_rect.w);
        let lbot = vec2<f32>(u.button_rect.x, u.button_rect.w);

        if (edge < tw) {
            ep = mix(top, rtop, edge / tw);
        } else if (edge < tw + th) {
            ep = mix(rtop, rbot, (edge - tw) / th);
        } else if (edge < 2.0 * tw + th) {
            ep = mix(rbot, lbot, (edge - tw - th) / tw);
        } else {
            ep = mix(lbot, top, (edge - 2.0 * tw - th) / th);
        }

        let dist = length(pixel - ep);
        energy += vec3<f32>(1.0, 0.85, 0.4) * exp(-dist * 0.8) * act;
        i += 1u;
    }

    // Procedural noise displacement
    let noise_val = fbm(vec3<f32>(pixel * 0.03, t * 1.5));
    let disp = (noise_val - 0.5) * u.thickness * 2.0 * act;
    let dd = d + disp;

    // === Three-layer border (from Phase 2: 4ff56f3) ===
    // Phase 2 used egui's solid Stroke renderer — recreate that visibility with sharp edges + narrow AA.

    // Layer 1 — Outer glow (wide, soft, like egui's StrokeKind::Outside)
    let glow_width = u.glow_spread + u.thickness;
    let glow_alpha = smoothstep(glow_width, glow_width - 2.0, d) * act * 0.7;

    // Layer 2 — Main stroke (sharp solid band, like egui's main Stroke)
    let stroke_half = u.thickness;
    let stroke_aa = 1.5; // narrow antialiasing band
    let stroke_alpha = smoothstep(stroke_half, stroke_half - stroke_aa, d);

    // Layer 3 — Inner highlight (thin, bright core)
    let inner_half = u.thickness * 0.5;
    let inner_aa = 1.0;
    let inner_alpha = smoothstep(inner_half, inner_half - inner_aa, d);
    let highlight_color = vec3<f32>(0.85, 1.0, 1.0);

    // Energy packets (RGB + alpha, so they show even on transparent bg)
    let energy_alpha = length(energy.rgb) * act * 0.5;

    // Composite: additive stacking of all layers
    var final_color =
        glow_alpha       * u.color.rgb * 0.5 +       // glow (dimmer, wider)
        stroke_alpha     * u.color.rgb +               // main stroke (solid)
        inner_alpha      * highlight_color * 0.5 +      // inner core (bright)
        energy_alpha     * vec3<f32>(1.0, 0.85, 0.4);  // energy packets

    // Final alpha: sum of all visible layers
    var final_alpha = glow_alpha + stroke_alpha + inner_alpha * 0.3 + energy_alpha;

    // Ensure disabled buttons still show a very subtle border
    final_alpha = max(final_alpha, 0.08 * (1.0 - act));

    return vec4<f32>(final_color, clamp(final_alpha, 0.0, 1.0));
}
