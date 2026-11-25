// Mask from depth (1 = near). u: rangeOn, from, to, feather, invert, biasDir (1 near, -1 far, 0 off), biasAmount
// textures: depth=1
layout(set = 0, binding = 7) uniform texture2D depth;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float d = texelFetch(sampler2D(depth, sNear), ivec2(gid), 0).r;
    float m = 1.0;
    if (u[0] > 0.5) {
        float lo = min(u[1], u[2]), hi = max(u[1], u[2]);
        float f = max(u[3], 0.002);
        float r = smoothstep(lo - f, lo + f * 0.25, d) * (1.0 - smoothstep(hi - f * 0.25, hi + f, d));
        if (lo <= 0.001) r = 1.0 - smoothstep(hi - f * 0.25, hi + f, d);
        if (hi >= 0.999) r = smoothstep(lo - f, lo + f * 0.25, d);
        if (u[4] > 0.5) r = 1.0 - r;
        m *= r;
    }
    if (abs(u[5]) > 0.5) {
        float w = u[5] > 0.0 ? d : 1.0 - d;
        w = smoothstep(0.0, 1.0, w);
        m *= mix(1.0, w, clamp(u[6], 0.0, 1.0));
    }
    imageStore(o, ivec2(gid), vec4(m, m, m, 1.0));
}
