// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = clamp(s.rgb, 0.0, 1.0);
        float l = luma(c);
        float w = smoothstep(u[0] - u[1], u[0] + u[1] + 1e-4, l);
        vec3 g = c;
        g = mix(vec3(l), g, u[6]);
        if (u[5] > 0.5) g = vec3(u[2], u[3], u[4]) * l;
        imageStore(o, ivec2(gid), vec4(g * w, 1.0));
}
