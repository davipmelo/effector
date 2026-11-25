// textures: a=1 lut=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D lut;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 c = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0), 0.0, 1.0);
        float n = float(uint(textureSize(sampler2D(lut, sNear), 0).x));
        vec3 r;
        r.r = textureLod(sampler2D(lut, sLin), vec2((c.r * (n - 1.0) + 0.5) / n, 0.5), 0.0).r;
        r.g = textureLod(sampler2D(lut, sLin), vec2((c.g * (n - 1.0) + 0.5) / n, 0.5), 0.0).g;
        r.b = textureLod(sampler2D(lut, sLin), vec2((c.b * (n - 1.0) + 0.5) / n, 0.5), 0.0).b;
        imageStore(o, ivec2(gid), vec4(r, c.a));
}
