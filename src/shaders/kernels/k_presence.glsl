// Detail › Presence: Dehaze, Clarity (large-radius midtone contrast) and Texture (fine detail).
// u: texture, clarity, dehaze (each -1..1)
// textures: src=1 fine blur=2 large blur=3 haze (blurred darkest channel)=4
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D bs;
layout(set = 0, binding = 9) uniform texture2D bl;
layout(set = 0, binding = 10) uniform texture2D dk;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    ivec2 p = ivec2(gid);
    vec4 s = texelFetch(sampler2D(a, sNear), p, 0);
    vec3 c = clamp(s.rgb, 0.0, 1.0);
    float L0 = luma(c);
    // Dehaze: remove (or add) an atmospheric veil estimated from the darkest channel
    float dh = u[2];
    if (dh > 0.001) {
        float dark = clamp(texelFetch(sampler2D(dk, sNear), p, 0).r, 0.0, 1.0);
        const float A = 0.96;
        float t = max(1.0 - dh * 0.92 * dark / A, 0.22);
        c = (c - A) / t + A;
        // keep mid-tones from falling too dark
        c = mix(c, c * (1.0 + 0.25 * dh), 0.5);
    } else if (dh < -0.001) {
        float k = -dh;
        vec3 veil = vec3(0.84, 0.86, 0.88);
        c = mix(c, veil, k * 0.55);
    }
    c = clamp(c, 0.0, 1.0);
    // Clarity: local contrast at a large radius, strongest in the mid-tones
    if (abs(u[1]) > 0.001) {
        float Lb = luma(clamp(texelFetch(sampler2D(bl, sNear), p, 0).rgb, 0.0, 1.0));
        float d = L0 - Lb;
        float L = luma(c);
        float w = 1.0 - pow(abs(2.0 * L - 1.0), 2.0);
        float dl = clamp(d * u[1] * 1.3 * w, -0.25, 0.25);
        c = clamp(c + dl, 0.0, 1.0);
    }
    // Texture: fine detail, gentle on strong edges (no halos)
    if (abs(u[0]) > 0.001) {
        float Lf = luma(clamp(texelFetch(sampler2D(bs, sNear), p, 0).rgb, 0.0, 1.0));
        float d = L0 - Lf;
        float dl = d * u[0] * 1.6 / (1.0 + abs(d) * 12.0);
        c = clamp(c + dl, 0.0, 1.0);
    }
    imageStore(o, p, vec4(c, s.a));
}
