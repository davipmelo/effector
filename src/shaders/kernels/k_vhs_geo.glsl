// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 p = vec2(gid) + 0.5;
        float unit = u[1], seed = u[0];
        float lineH = max(1.0, u[11]);
        float line = floor(p.y / lineH);
        float yn = p.y / size.y;
        float dx = 0.0;
        if (u[6] > 0.5) {
            dx += sin(yn * u[8] * 6.2831 + seed) * u[7] * unit
                + (vnoise(vec2(line * 0.03, seed), seed) - 0.5) * u[7] * unit * 1.2;
        }
        float band = 0.0;
        if (u[2] > 0.5) {
            dx += (hash12(vec2(line, seed)) - 0.5) * u[3] * unit * 0.18;
            float d = abs(yn - u[4]) / max(0.004, u[5]);
            band = smoothstep(1.0, 0.0, d);
            dx += band * (vnoise(vec2(line * 0.35, seed + 3.0), seed) - 0.25) * u[3] * unit * 3.5;
        }
        // head switching: the bottom lines are pulled sideways
        float hs = 0.0;
        if (u[9] > 0.5) {
            float h0 = 1.0 - u[10];
            hs = smoothstep(h0, h0 + 0.004, yn);
            float k = (yn - h0) / max(1e-3, u[10]);
            dx += hs * (18.0 + 26.0 * k + (hash12(vec2(line, seed + 21.0)) - 0.5) * 14.0) * unit;
        }
        vec2 uv = (p + vec2(dx, 0.0)) / size;
        vec3 c = textureLod(sampler2D(a, sLin), uv, 0.0).rgb;
        if (uv.x < 0.0 || uv.x > 1.0) c = vec3(0.02);
        imageStore(o, ivec2(gid), vec4(c, 1.0));
}
