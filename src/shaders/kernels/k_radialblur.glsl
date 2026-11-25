// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        float asp = size.x / size.y;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 c = vec2(u[2], u[3]);
        float m = edgeMask(uv, asp, c, u[4], u[5], u[6]);
        vec2 q = (uv - c) * vec2(asp, 1.0);
        float amt = u[1] * m * u[7];
        const int N = 28;
        vec3 acc = vec3(0.0);
        float jit = hash12(vec2(gid) + u[7] * 7.0);
        int mode = int(u[0]);
        for (int i = 0; i < N; i++) {
            float t = (float(i) + jit) / float(N) - 0.5;      // centred on the pixel
            vec2 sq = q;
            if (mode == 0 || mode == 2) sq *= 1.0 + t * amt * 0.5;
            if (mode == 1 || mode == 2) {
                float an = t * amt * 0.6;
                float cs = cos(an), sn = sin(an);
                sq = vec2(cs * sq.x - sn * sq.y, sn * sq.x + cs * sq.y);
            }
            acc += textureLod(sampler2D(a, sLin), c + sq / vec2(asp, 1.0), 0.0).rgb;
        }
        imageStore(o, ivec2(gid), vec4(acc / float(N), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
