// textures: a=1 lut=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture3D lut;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float n = float(uint(textureSize(sampler3D(lut, sNear), 0).x));
        vec3 c = clamp(s.rgb, 0.0, 1.0);
        vec3 uvw = c * (n - 1.0) / n + 0.5 / n;
        vec3 r = textureLod(sampler3D(lut, sLin), uvw, 0.0).rgb;
        imageStore(o, ivec2(gid), vec4(mix(s.rgb, r, u[0]), s.a));
}
