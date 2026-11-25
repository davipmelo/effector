// Film halation: a warm glow added in linear light around highlights. The tight scale uses the
// inner colour (orange), the wide scales the outer colour (deep red), so cores stay white and the
// fringe turns red as it spreads. Optional neutral bloom from the same source.
// u: intensity, spread(0..1), outer rgb (linear), inner rgb (linear), bloom
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D g1;
layout(set = 0, binding = 9) uniform texture2D g2;
layout(set = 0, binding = 10) uniform texture2D g3;
// Soft add: ~base + add for small values, approaches 1 without a hard clip (base untouched when add = 0).
vec3 softAdd(vec3 base, vec3 add) {
    vec3 room = max(1.0 - base, 1e-4);
    return base + room * (1.0 - exp(-max(add, 0.0) / room));
}
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec4 s1 = textureLod(sampler2D(g1, sLin), uv, 0.0);
    vec4 s2 = textureLod(sampler2D(g2, sLin), uv, 0.0);
    vec4 s3 = textureLod(sampler2D(g3, sLin), uv, 0.0);
    float sp = clamp(u[1], 0.0, 1.0);
    vec3 wt = mix(vec3(0.70, 0.25, 0.05), vec3(0.15, 0.35, 0.50), sp);
    vec3 outer = vec3(u[2], u[3], u[4]);
    vec3 inner = vec3(u[5], u[6], u[7]);
    vec3 halo = inner * (s1.a * wt.x) + outer * (s2.a * wt.y + s3.a * wt.z);
    vec3 bloom = (s1.rgb * 0.45 + s2.rgb * 0.35 + s3.rgb * 0.2) * u[8];
    vec3 lin = softAdd(toLinear(clamp(b.rgb, 0.0, 1.0)), halo * u[0] + bloom);
    imageStore(o, ivec2(gid), vec4(toSRGB(clamp(lin, 0.0, 1.0)), b.a));
}
