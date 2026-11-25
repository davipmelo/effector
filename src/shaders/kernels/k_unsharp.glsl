// textures: a=1 b=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 bl = texelFetch(sampler2D(b, sNear), ivec2(gid), 0).rgb;
        vec3 d = s.rgb - bl;
        if (u[2] > 0.5) d = vec3(luma(d));
        float m = smoothstep(u[1], u[1] * 2.0 + 1e-4, abs(luma(s.rgb - bl)));
        imageStore(o, ivec2(gid), vec4(clamp(s.rgb + d * u[0] * m, 0.0, 1.0), s.a));
}
