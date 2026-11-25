// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float l = neutralGray(c.rgb);
        float s = max(u[1], 0.0005);
        float t = smoothstep(u[0] - s, u[0] + s, l);
        imageStore(o, ivec2(gid), vec4(mix(vec3(u[2], u[3], u[4]), vec3(u[5], u[6], u[7]), t), c.a));
}
