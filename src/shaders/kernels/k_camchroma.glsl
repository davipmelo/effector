// Video chroma: colour is recorded at much lower horizontal resolution and a little late.
// u: 0 bleed px, 1 delay px, 2 chroma noise, 3 seed, 4 luma softness px
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 yiq = rgb2yiq(s.rgb);
    // luma: slight horizontal softness
    if (u[4] > 0.05) {
        float yy = 0.0, w = 0.0;
        for (int i = -3; i <= 3; i++) {
            float k = exp(-float(i * i) / max(u[4] * u[4], 0.01));
            yy += rgb2yiq(textureLod(sampler2D(a, sLin), uv + vec2(float(i) * 0.5 / size.x, 0.0), 0.0).rgb).x * k;
            w += k;
        }
        yiq.x = yy / w;
    }
    vec2 iq = vec2(0.0);
    float wsum = 0.0;
    const int N = 12;
    for (int i = 0; i < N; i++) {
        float t = (float(i) / float(N - 1) - 0.5) * u[0];
        vec3 q = rgb2yiq(textureLod(sampler2D(a, sLin), uv + vec2((t - u[1]) / size.x, 0.0), 0.0).rgb);
        float w = 1.0 - abs(float(i) / float(N - 1) - 0.5);
        iq += q.yz * w;
        wsum += w;
    }
    iq /= wsum;
    vec2 p = vec2(gid);
    vec2 cn = vec2(vnoise(p * vec2(0.08, 0.6), u[3]) - 0.5, vnoise(p * vec2(0.08, 0.6) + 7.0, u[3] + 2.0) - 0.5);
    iq += cn * u[2] * 0.25;
    imageStore(o, ivec2(gid), vec4(clamp(yiq2rgb(vec3(yiq.x, iq)), 0.0, 1.0), s.a));
}
