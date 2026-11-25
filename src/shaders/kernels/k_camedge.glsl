// Camcorder edge enhancement: luma-only sharpening with ringing (the bright halo outlines of DV /
// Hi8 / CCTV) and coloured fringes on vertical edges from misaligned chroma.
// u: 0 sharpen, 1 ringing, 2 fringe, 3 fringe width px, 4 halo brightness bias
// textures: a=1 b1=2 (small blur) b2=3 (large blur)
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b1;
layout(set = 0, binding = 9) uniform texture2D b2;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 c = s.rgb;
    float y = luma(c);
    float y1 = luma(textureLod(sampler2D(b1, sLin), uv, 0.0).rgb);
    float y2 = luma(textureLod(sampler2D(b2, sLin), uv, 0.0).rgb);
    float hp = y - y1;
    float ring = y1 - y2;
    float boost = u[0] * hp * 2.2 + u[1] * ring * 2.0;
    // cameras clip the dark undershoot less than the bright overshoot: halos read as white outlines
    boost = boost > 0.0 ? boost * (1.0 + u[4]) : boost * (1.0 - u[4] * 0.5);
    c += boost;
    // chroma misregistration: one side of an edge goes green/cyan, the other magenta/orange
    float dx = u[3] / size.x;
    float gx = luma(textureLod(sampler2D(b1, sLin), uv + vec2(dx, 0.0), 0.0).rgb) - luma(textureLod(sampler2D(b1, sLin), uv - vec2(dx, 0.0), 0.0).rgb);
    c += u[2] * vec3(-0.6 * gx, 0.75 * gx, 0.35 * gx) * 1.6;
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.15), s.a));
}
