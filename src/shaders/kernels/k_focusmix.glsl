// textures: base=1 b1=2 b2=3
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D b1;
layout(set = 0, binding = 9) uniform texture2D b2;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        int mode = int(u[0]);
        float m;
        vec2 c = vec2(u[1], u[2]);
        if (mode == 0) {
            float an = u[5] * 0.0174533;
            vec2 nrm = vec2(-sin(an), cos(an));
            float d = abs(dot((uv - c) * vec2(size.x / size.y, 1.0), nrm));
            m = smoothstep(u[3] * 0.5, u[3] * 0.5 + max(0.01, u[4]), d);
        } else if (mode == 1) {
            float d = length((uv - c) * vec2(size.x / size.y, 1.0));
            m = smoothstep(u[3] * 0.5, u[3] * 0.5 + max(0.01, u[4]), d);
        } else {
            m = 1.0;
        }
        vec4 s = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        vec3 x1 = textureLod(sampler2D(b1, sLin), uv, 0.0).rgb, x2 = textureLod(sampler2D(b2, sLin), uv, 0.0).rgb;
        vec3 r = m < 0.5 ? mix(s.rgb, x1, m * 2.0) : mix(x1, x2, (m - 0.5) * 2.0);
        r = mix(vec3(luma(r)), r, u[6]);
        imageStore(o, ivec2(gid), vec4(clamp(r, 0.0, 1.0), s.a));
}
