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
    // TODO(dead field — never read in the shader): the expanded quad/scissor come
    // from the paint callback rect + scissor rect in the host, not from this field,
    // yet it is written into every uniform buffer.
    // Suggested fix: use it for the padding band or drop it from the struct in
    // electric.wgsl and ElectricButtonUniforms.
    glow_padding: f32,
    /// Animation time
    time: f32,
    /// Active multiplier
    is_active: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;
// TODO(dead resource — sampler never sampled): `s` is never sampled by the shader,
// yet it forces a sampler to be created and bound per button every frame.
// Suggested fix: remove this binding (and the matching bind group layout entry +
// sampler creation in electric_button_renderer.rs).
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

// TODO(dead computation — only one hash component is used): `s.y` and `s.z` are
// computed but never read, and the result is a single linear projection of the
// 3D point, so the 3D noise is heavily correlated in the z direction.
// Suggested fix: compute only the used dot product, or hash (s.x, s.y, s.z) in a
// 3D-consistent way (e.g. 3 separate dot products mixed, or a proper 3D value hash
// like a tri-planar/improved perlin hash).
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

// TODO(bug — SDF is not a rounded-rect SDF): this computes the distance to the
// UNROUNDED rect and subtracts the radius, so inside the rect it returns a constant
// -radius (instead of the signed distance to the rounded edge, e.g. -2.0 at the
// edge). The zero-crossing lands `radius` px OUTSIDE the unrounded edge, so the
// whole interior fills with the full stroke/glow/highlight band (alpha saturates
// at 1.0) and the border floats in the padding ring.
// Suggested fix: canonical rounded-rect SDF, e.g.
//   let center = 0.5 * (rect.xy + rect.zw);
//   let half = 0.5 * (rect.zw - rect.xy);
//   let q = abs(p - center) - (half - radius);
//   return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
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
        // TODO(bug — energy packets never travel): `offset - perimeter * (offset / perimeter)`
        // cancels to ~0 in float math, so every packet is stuck at the top-left corner
        // (edge ≈ 0 → top edge, position = top corner).
        // Suggested fix: `let edge = offset % perimeter;`
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

        // TODO(minor — act²): `energy` is already multiplied by `act` inside the loop,
        // and `energy_alpha` below multiplies by `act` again.
        // Suggested fix: multiply `act` exactly once (either here or in energy_alpha).
        let dist = length(pixel - ep);
        energy += vec3<f32>(1.0, 0.85, 0.4) * exp(-dist * 0.8) * act;
        i += 1u;
    }

    // Procedural noise displacement
    let noise_val = fbm(vec3<f32>(pixel * 0.03, t * 1.5));
    let disp = (noise_val - 0.5) * u.thickness * 2.0 * act;
    // TODO(major — displacement never reaches the output): `dd` is computed but never
    // used (glow/stroke/highlight bands all read `d`, and the energy distance uses
    // the raw pixel-to-packet distance, not `dd`), so the procedural lightning is
    // computation with no visual effect.
    // Suggested fix: use `dd` in the glow/stroke/highlight bands (and/or pass `dd`
    // into the energy distance), or remove the dead fbm/disp/dd chain.
    let dd = d + disp;

    // === Three-layer border (from Phase 2: 4ff56f3) ===

    // Layer 1 — Outer glow (wide, soft)
    let glow_width = u.glow_spread + u.thickness;
    // TODO(style — reversed smoothstep is legal here but unclear): WGSL defines
    // smoothstep for edge0 > edge1 (for signed types), so this works, but
    // `1.0 - smoothstep(0.0, glow_width, d)` is clearer and equivalent.
    let glow_alpha = smoothstep(glow_width, 0.0, d) * act * 1.0;

    // Layer 2 — Main stroke (wide solid band)
    let stroke_half = u.thickness;
    // TODO(style — band too wide): 6.0px width gives a 12px-wide band (d ∈ [-4, +8])
    // around the border, far wider than the "thin electric border" in the design doc
    // and the ~1.5px transitions noted in evidence/border-visibility-findings.md.
    // Suggested fix: narrow this to ~1.5-2px (and consider making it configurable),
    // or combine it with the SDF/displacement fixes above.
    let stroke_width = 6.0; // wide stroke band for visibility
    let stroke_alpha = smoothstep(stroke_half + stroke_width, stroke_half - stroke_width, d);
    let stroke_color = vec3<f32>(1.0, 1.0, 1.0); // bright white stroke

    // Layer 3 — Inner highlight
    let inner_half = u.thickness * 0.3;
    let inner_alpha = smoothstep(inner_half, -inner_half, d);
    let highlight_color = vec3<f32>(0.9, 1.0, 1.0);

    // Energy packets
    let energy_alpha = length(energy.rgb) * act * 0.3;

    // Composite
    var final_color =
        glow_alpha * u.color.rgb * 0.3 +
        stroke_alpha * stroke_color +
        inner_alpha * highlight_color * 0.8 +
        energy_alpha * vec3<f32>(1.0, 0.85, 0.4);

    // Final alpha
    var final_alpha = glow_alpha + stroke_alpha + inner_alpha * 0.5 + energy_alpha;

    // Disabled minimum
    final_alpha = max(final_alpha, 0.1 * (1.0 - act));

    return vec4<f32>(final_color, clamp(final_alpha, 0.0, 1.0));
}
