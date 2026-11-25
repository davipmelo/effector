// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 os = vec2(imageSize(o));
        vec2 is = vec2(uint(textureSize(sampler2D(a, sNear), 0).x), uint(textureSize(sampler2D(a, sNear), 0).y));
        float s = max(os.x / is.x, os.y / is.y);
        vec2 p = (vec2(gid) + 0.5 - os * 0.5) / s + is * 0.5;
        imageStore(o, ivec2(gid), vec4(textureLod(sampler2D(a, sLin), p / is, 0.0).rgb, 1.0));
}
