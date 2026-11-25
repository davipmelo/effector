// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec3 c0 = texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb;
        vec3 acc = vec3(0.0); float ws = 0.0;
        float th = max(0.01, u[1]);
        for (int y = -4; y <= 4; y++) {
            for (int x = -4; x <= 4; x++) {
                vec2 o2 = vec2(x, y) / 4.0 * u[0];
                vec3 s = textureLod(sampler2D(a, sLin), uv + o2 / size, 0.0).rgb;
                float ds = length(s - c0);
                float w = exp(-dot(vec2(x, y), vec2(x, y)) / 18.0) * exp(-ds * ds / (th * th));
                acc += s * w; ws += w;
            }
        }
        imageStore(o, ivec2(gid), vec4(acc / ws, texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
