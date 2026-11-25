// textures: a=1 b=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec3 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb;
        float area = luma(textureLod(sampler2D(b, sLin), ((vec2(gid) + 0.5) / vec2(imageSize(o))), 0.0).rgb);
        imageStore(o, ivec2(gid), vec4(c * (1.0 - u[0] * smoothstep(0.08, 0.45, area)), 1.0));
}
