// Fog / haze that thickens with distance (depth map; falls back to "higher in the frame = farther").
// u: 0 density, 1 curve, 2-4 colour, 5 start distance, 6 height amount, 7 height position, 8 noise,
//    9 noise scale px, 10 seed, 11 glow, 12 has depth, 13 lift blacks
// textures: a=1 depth=2 glow=3 (blurred image)
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D depth;
layout(set = 0, binding = 9) uniform texture2D glowt;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float d = u[12] > 0.5 ? texelFetch(sampler2D(depth, sNear), ivec2(gid), 0).r : uv.y * 0.85;
    float far = clamp((1.0 - d - u[5]) / max(1.0 - u[5], 0.02), 0.0, 1.0);
    float f = u[0] * pow(far, max(u[1], 0.1));
    // ground fog: thicker low in the frame
    f *= mix(1.0, smoothstep(u[7] - 0.35, u[7] + 0.35, uv.y), u[6]);
    if (u[8] > 0.001) {
        float n = fbm(vec2(gid) / max(u[9], 1.0), u[10], 4);
        f *= mix(1.0, 0.35 + 1.3 * n, u[8]);
    }
    f = clamp(f, 0.0, 1.0);
    vec3 fogc = vec3(u[2], u[3], u[4]);
    vec3 g = textureLod(sampler2D(glowt, sLin), uv, 0.0).rgb;
    vec3 c = mix(s.rgb, fogc + g * u[11] * 0.6, f);
    // light scattered in the haze lifts the shadows a little everywhere
    c = c + (fogc - c) * u[13] * f * 0.3;
    c += g * u[11] * f * 0.35;
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
