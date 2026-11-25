// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = clamp(s.rgb, 0.0, 1.0);
        c = pow(c, vec3(1.0 / max(0.05, u[3])));
        c = c + u[0] / 100.0;
        float k = u[1] / 100.0;
        c = (c - 0.5) * (k >= 0.0 ? 1.0 + k * 2.0 : 1.0 + k) + 0.5;
        c = clamp(c, 0.0, 1.0);
        if (u[2] > 0.5) c = vec3(neutralGray(c));
        imageStore(o, ivec2(gid), vec4(c, s.a));
}
