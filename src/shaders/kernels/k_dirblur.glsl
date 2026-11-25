// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        const int N = 32;
        vec3 acc = vec3(0.0); float ws = 0.0;
        float jit = hash12(vec2(gid) + u[0]);
        for (int i = 0; i < N; i++) {
            float t = (float(i) + jit) / float(N);
            if (u[2] > 0.5) t -= 0.5;
            float w = u[3] > 0.5 ? (1.0 - abs(t) * (u[2] > 0.5 ? 2.0 : 1.0)) : 1.0;
            acc += textureLod(sampler2D(a, sLin), uv + vec2(u[0], u[1]) * t / size, 0.0).rgb * w;
            ws += w;
        }
        imageStore(o, ivec2(gid), vec4(acc / max(ws, 1e-4), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
