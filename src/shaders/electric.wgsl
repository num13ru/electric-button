// WGSL shader for procedural electric lightning border.
//
// Draws a rounded-rectangle SDF with time-varying procedural
// noise displacement for the lightning effect. Energy packets
// travel along the perimeter creating bright spots.

struct Uniforms {
    /// Button body rect in physical pixels (min_x, min_y, max_x, max_y)
    button_rect: vec4<f32>,
    /// Expanded callback rect in physical pixels (min_x, min_y, max_x, max_y)
    viewport: vec4<f32>,
    /// Corner radius in pixels
    radius: f32,
    /// Border thickness in pixels
    thickness: f32,
    /// Border color (RGBA)
    color: vec4<f32>,
    /// Glow spread in pixels
    glow_spread: f32,
    /// Animation time
    time: f32,
    /// Active multiplier
    is_active: f32,
    /// State-dependent brightness multiplier
    intensity: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

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
    let s = dot(p, vec3<f32>(127.1, 311.7, 74.7));
    return fract(sin(s) * 43758.5453123);
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
    let center = 0.5 * (rect.xy + rect.zw);
    let half = 0.5 * (rect.zw - rect.xy);
    let q = abs(p - center) - (half - radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    // Convert UV to physical-pixel coords within the expanded callback rect.
    let pixel = u.viewport.xy + input.uv * (u.viewport.zw - u.viewport.xy);

    // SDF
    let d = sdf_rounded_rect(pixel, u.button_rect, u.radius);

    // Time
    let t = u.time;
    let act = u.is_active;
    let intensity = u.intensity;

    // Traveling energy packets along the perimeter
    let tw = u.button_rect.z - u.button_rect.x;
    let th = u.button_rect.w - u.button_rect.y;
    let perimeter = 2.0 * tw + 2.0 * th;
    let num_packets = 3.0;
    var energy = vec3<f32>(0.0);
    var i = 0u;
    while (i < 3u) {
        let offset = t * 30.0 + f32(i) * (perimeter / num_packets);
        let edge = fract(offset / perimeter) * perimeter;

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
        energy += mix(u.color.rgb, vec3<f32>(1.0, 0.85, 0.4), 0.45) * exp(-dist * 0.8) * act * intensity;
        i += 1u;
    }

    // Procedural noise displacement
    let noise_val = fbm(vec3<f32>(pixel * 0.03, t * 1.5));
    let disp = (noise_val - 0.5) * u.thickness * 2.0 * act * intensity;
    let dd = d + disp;

    // Layer 1 — Outer glow (outside the border only)
    let glow_width = u.glow_spread + u.thickness;
    let glow_alpha = (1.0 - smoothstep(0.0, glow_width, dd)) * act * intensity * step(0.0, dd);

    // Layer 2 — Main stroke (thin outline band around the boundary)
    let stroke_aa = 1.5;
    let stroke_alpha = (1.0 - smoothstep(u.thickness - stroke_aa, u.thickness + stroke_aa, abs(dd))) * act * intensity;
    let stroke_color = mix(u.color.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.35);

    // Layer 3 — Inner highlight (thin bright core on the boundary)
    let highlight_alpha = (1.0 - smoothstep(0.0, 1.0, abs(dd))) * act * intensity;
    let highlight_color = mix(u.color.rgb, vec3<f32>(0.95, 1.0, 1.0), 0.65);

    // Energy packets (already scaled by act inside the loop)
    let energy_alpha = length(energy.rgb) * 0.3;

    // Composite with sequential "over" blending (color must NOT be scaled
    // by alpha — blending applies src alpha again, double-dimming the glow).
    let glow_color = mix(u.color.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.35);
    let energy_color = mix(u.color.rgb, vec3<f32>(1.0, 0.85, 0.4), 0.45);

    var rgb = glow_color;
    var a = glow_alpha;

    rgb = stroke_color * stroke_alpha + rgb * (1.0 - stroke_alpha);
    a = stroke_alpha + a * (1.0 - stroke_alpha);

    let hi_alpha = highlight_alpha * 0.8;
    rgb = highlight_color * hi_alpha + rgb * (1.0 - hi_alpha);
    a = hi_alpha + a * (1.0 - hi_alpha);

    rgb = energy_color * energy_alpha + rgb * (1.0 - energy_alpha);
    a = energy_alpha + a * (1.0 - energy_alpha);

    // Disabled minimum: dim ghost outline in neutral gray
    let ghost_stroke = (1.0 - smoothstep(u.thickness - stroke_aa, u.thickness + stroke_aa, abs(dd))) * (1.0 - act) * 0.35;
    rgb = vec3<f32>(0.75, 0.75, 0.8) * ghost_stroke + rgb * (1.0 - ghost_stroke);
    a = ghost_stroke + a * (1.0 - ghost_stroke);

    return vec4<f32>(rgb, clamp(a, 0.0, 1.0));
}
