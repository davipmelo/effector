// textures: base=1 g1=2 g2=3 g3=4
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D g1;
layout(set = 0, binding = 9) uniform texture2D g2;
layout(set = 0, binding = 10) uniform texture2D g3;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec3 g = textureLod(sampler2D(g1, sLin), uv, 0.0).rgb * u[2] + textureLod(sampler2D(g2, sLin), uv, 0.0).rgb * u[3] + textureLod(sampler2D(g3, sLin), uv, 0.0).rgb * u[4];
        g *= vec3(u[5], u[6], u[7]) * u[0];
        if (u[8] > 0.5) g = vec3(luma(g));
        vec3 r;
        int m = int(u[1]);
        if (m == 1) r = b.rgb + g;
        else if (m == 2) r = blendSoftLight(clamp(b.rgb, 0.0, 1.0), clamp(0.5 + g * 0.5, 0.0, 1.0));
        else r = 1.0 - (1.0 - clamp(b.rgb, 0.0, 1.0)) * (1.0 - clamp(g, 0.0, 1.0));
        imageStore(o, ivec2(gid), vec4(clamp(r, 0.0, 1.0), b.a));
}
