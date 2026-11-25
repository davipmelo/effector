// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        float l = luma(clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb, 0.0, 1.0));
        imageStore(o, ivec2(gid), vec4(vec3(smoothstep(0.8, 0.97, l)), 1.0));
}
