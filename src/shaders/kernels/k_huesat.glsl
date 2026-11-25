// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 src = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 lch = srgb2oklch(src.rgb);
        if (u[3] > 0.5) {
            vec3 ref = srgb2oklch(hsl2rgb(vec3(u[4] / 360.0, 1.0, 0.5)));
            lch.z = ref.z;
            lch.y = u[5] / 100.0 * 0.26;
        } else {
            lch.z += u[0] * 0.0174533;
            float s = u[1] / 100.0;
            lch.y *= max(0.0, 1.0 + s);
        }
        vec3 c = oklch2srgb(lch);
        float L = u[2] / 100.0;
        c = L >= 0.0 ? mix(c, vec3(1.0), L) : c * (1.0 + L);
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), src.a));
}
