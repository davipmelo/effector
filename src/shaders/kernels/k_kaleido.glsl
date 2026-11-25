// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        float asp = size.x / size.y;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 q = (uv - vec2(u[2], u[3])) * vec2(asp, 1.0);
        float r = length(q) / max(0.05, u[4]);
        float N = max(2.0, floor(u[0]));
        float seg = 6.2831853 / N;
        if (u[9] > 0.5) {
            float k = r * u[9];
            r = abs(fract(k * 0.5) * 2.0 - 1.0) * 0.5;
        }
        vec3 col;
        for (int ch = 0; ch < 3; ch++) {
            float th = atan(q.y, q.x) + u[1] + u[5] * r + (float(ch) - 1.0) * u[7] * seg * 0.12;
            float k = floor(th / seg);
            float tm = th - k * seg;
            if (u[6] > 0.5 && fmodc(abs(k), 2.0) > 0.5) tm = seg - tm;
            vec2 sp = vec2(cos(tm), sin(tm)) * r;
            vec2 suv = vec2(u[10], u[11]) + sp / vec2(asp, 1.0);
            col[ch] = textureLod(sampler2D(soft, sMir), suv, 0.0)[ch];
        }
        vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        imageStore(o, ivec2(gid), vec4(mix(base.rgb, col, clamp(u[8], 0.0, 1.0)), base.a));
}
