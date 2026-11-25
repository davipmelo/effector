// LED dot-matrix: one LED per cell of the downscaled image.
// u: cell px, dot size, dot height, threshold, levels, mode(0 mono, 1 colour), unlit, contrast,
//    grid w, grid h, led rgb, panel rgb
layout(set = 0, binding = 7) uniform texture2D g;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 p = vec2(gid) + 0.5;
    float C = max(u[0], 1.0);
    vec2 cellId = floor(p / C);
    vec2 f = p / C - cellId - 0.5;               // -0.5..0.5
    ivec2 gi = clamp(ivec2(cellId), ivec2(0), ivec2(int(u[8]) - 1, int(u[9]) - 1));
    vec4 s = texelFetch(sampler2D(g, sNear), gi, 0);
    vec3 c = clamp(s.rgb, 0.0, 1.0);
    float l = luma(c);
    l = clamp((l - 0.5) * (1.0 + u[7]) + 0.5, 0.0, 1.0);
    float lit = l < u[3] ? 0.0 : l;
    float lv = max(u[4], 2.0);
    lit = floor(lit * (lv - 1.0) + 0.5) / (lv - 1.0);
    // dot shape: circle, stretched vertically by dot height (tall dots merge into bars)
    float rad = u[1] * 0.5;
    float h = max(u[2], 1.0);
    // a tall dot overlaps the cells above / below: use the cell's own value, clipped to its row band
    float dist = length(vec2(f.x, max(abs(f.y) - (h - 1.0) * 0.5 * rad, 0.0)));
    float aa = 1.5 / C;
    float shape = 1.0 - smoothstep(rad - aa, rad + aa, dist);
    vec3 ledCol = u[5] > 0.5 ? c / max(max(c.r, max(c.g, c.b)), 1e-3) : vec3(u[10], u[11], u[12]);
    vec3 panel = vec3(u[13], u[14], u[15]);
    vec3 on = ledCol * lit;
    vec3 off = mix(panel, ledCol, 0.18) * u[6];
    vec3 col = mix(panel, max(on, off), shape);
    imageStore(o, ivec2(gid), vec4(col, 1.0));
}
