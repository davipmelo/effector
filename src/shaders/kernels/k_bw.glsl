// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 src = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = clamp(src.rgb, 0.0, 1.0);
        float mn = min(c.r, min(c.g, c.b));
        float mx = max(c.r, max(c.g, c.b));
        float md = c.r + c.g + c.b - mn - mx;
        float wR = u[0] / 100.0, wY = u[1] / 100.0, wG = u[2] / 100.0;
        float wC = u[3] / 100.0, wB = u[4] / 100.0, wM = u[5] / 100.0;
        float wp, ws;
        if (c.r >= c.g && c.r >= c.b) { wp = wR; ws = c.g >= c.b ? wY : wM; }
        else if (c.g >= c.r && c.g >= c.b) { wp = wG; ws = c.r >= c.b ? wY : wC; }
        else { wp = wB; ws = c.r >= c.g ? wM : wC; }
        float g = mn + (md - mn) * ws + (mx - md) * wp;
        float k = u[6] / 100.0;
        g = (g - 0.5) * (1.0 + k) + 0.5;
        g = clamp(g, 0.0, 1.0);
        imageStore(o, ivec2(gid), vec4(g, g, g, src.a));
}
