// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = clamp(s.rgb, 0.0, 1.0);
        float l = luma(c);
        imageStore(o, ivec2(gid), vec4(c * (1.0 + u[0] * 4.0 * smoothstep(0.7, 1.0, l)), s.a));
}
