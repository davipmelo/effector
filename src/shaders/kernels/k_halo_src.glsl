// Halation / bloom source (half-res). rgb = linear light above the threshold (for the bloom part),
// a = halation drive: film halation re-exposes the red-sensitive layer, so it is driven by the red
// channel (u[3] = 0) or by luminance (u[3] = 1).
// u: threshold, knee, unused, source
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec3 c = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb, 0.0, 1.0);
    vec3 lin = toLinear(c);
    float key = u[3] > 0.5 ? luma(c) : max(c.r, luma(c));
    float w = smoothstep(u[0] - u[1], u[0] + u[1] + 1e-4, key);
    float drive = (u[3] > 0.5 ? luma(lin) : lin.r) * w;
    imageStore(o, ivec2(gid), vec4(lin * w, drive));
}
