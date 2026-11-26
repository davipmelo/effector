// Diffusion filter, step 2 (Pro-Mist, Glimmerglass, Cinebloom…). Physically based and energy
// conserving in linear light: each pixel loses the share it scatters (k·s·lin) and receives the
// scattered light of its neighbourhood (k · weighted blurs). Highlights bloom, nearby shadows lift a
// little (as with the real glass), the overall exposure and colour stay the same.
// u: 0 k (scatter amount), 1 highlight bias, 2-4 weights of the tight / medium / wide blurs,
//    5 detail softening (0..1, mixes toward a small blur of the image)
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D g1;
layout(set = 0, binding = 9) uniform texture2D g2;
layout(set = 0, binding = 10) uniform texture2D g3;
layout(set = 0, binding = 11) uniform texture2D soft;
float scatterShare(vec3 c, float bias) {
    return mix(1.0, smoothstep(0.30, 0.95, luma(c)), bias);
}
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec3 c = clamp(b.rgb, 0.0, 1.0);
    vec3 lin = toLinear(c);
    float k = u[0];
    vec3 scattered = textureLod(sampler2D(g1, sLin), uv, 0.0).rgb * u[2]
                   + textureLod(sampler2D(g2, sLin), uv, 0.0).rgb * u[3]
                   + textureLod(sampler2D(g3, sLin), uv, 0.0).rgb * u[4];
    vec3 outL = lin - k * scatterShare(c, u[1]) * lin + k * scattered;
    if (u[5] > 0.0001) {
        vec3 s = textureLod(sampler2D(soft, sLin), uv, 0.0).rgb;
        outL += (s - lin) * u[5];
    }
    imageStore(o, ivec2(gid), vec4(toSRGB(clamp(outL, 0.0, 1.0)), b.a));
}
