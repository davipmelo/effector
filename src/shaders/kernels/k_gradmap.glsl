// textures: a=1 lut=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D lut;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float g = neutralGray(clamp(s.rgb, 0.0, 1.0));
        g = clamp((g - u[1]) / max(1e-3, u[2] - u[1]), 0.0, 1.0);
        if (u[3] > 0.5) g = 1.0 - g;
        float n = float(uint(textureSize(sampler2D(lut, sNear), 0).x));
        vec3 c = textureLod(sampler2D(lut, sLin), vec2((g * (n - 1.0) + 0.5) / n, 0.5), 0.0).rgb;
        imageStore(o, ivec2(gid), vec4(mix(s.rgb, c, u[0]), s.a));
}
