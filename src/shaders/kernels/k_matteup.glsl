// Depth map (whole image, model resolution) -> render size of the cropped image, with a joint
// bilateral filter guided by the photo so depth edges snap to the real object edges.
// u: crop x, y, w, h (normalised), low-res w, h
// textures: depth=1 guide=2
layout(set = 0, binding = 7) uniform texture2D depth;
layout(set = 0, binding = 8) uniform texture2D guide;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec2 crop0 = vec2(u[0], u[1]), cropS = vec2(u[2], u[3]);
    vec2 lowS = vec2(u[4], u[5]);
    vec2 iuv = crop0 + uv * cropS;                  // image space
    vec3 cp = textureLod(sampler2D(guide, sLin), uv, 0.0).rgb;
    vec2 lp = iuv * lowS - 0.5;                      // low-res texel space
    vec2 base = floor(lp);
    float sum = 0.0, wsum = 0.0;
    for (int j = -2; j <= 3; j++) {
        for (int i = -2; i <= 3; i++) {
            vec2 q = base + vec2(float(i), float(j));
            vec2 qi = (q + 0.5) / lowS;               // image uv of that texel centre
            float dq = textureLod(sampler2D(depth, sNear), qi, 0.0).r;
            vec2 d = (q - lp);
            float ws = exp(-dot(d, d) / 2.2);
            vec2 guv = clamp((qi - crop0) / cropS, vec2(0.0), vec2(1.0));
            vec3 cq = textureLod(sampler2D(guide, sLin), guv, 0.0).rgb;
            vec3 dc = cq - cp;
            float wc = exp(-dot(dc, dc) / (2.0 * 0.012));
            float w = ws * (0.02 + wc);
            sum += dq * w;
            wsum += w;
        }
    }
    float v = sum / max(wsum, 1e-6);
    imageStore(o, ivec2(gid), vec4(v, v, v, 1.0));
}
