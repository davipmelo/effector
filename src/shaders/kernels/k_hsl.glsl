// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 src = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 lch = srgb2oklch(src.rgb);
        // band centres (OKLCh hue, degrees): red orange yellow green aqua blue purple magenta
        const float centers[8] = float[](29.0, 58.0, 108.0, 142.0, 195.0, 264.0, 295.0, 330.0);
        float deg = lch.z * 57.29578;
        deg = deg < 0.0 ? deg + 360.0 : deg;
        float dh = 0, ds = 0, dl = 0;
        for (int i = 0; i < 8; i++) {
            float c0 = centers[i];
            float cPrev = centers[(i + 7) % 8]; float cNext = centers[(i + 1) % 8];
            float dPrev = fmodc(c0 - cPrev + 360.0, 360.0);
            float dNext = fmodc(cNext - c0 + 360.0, 360.0);
            float d = fmodc(deg - c0 + 540.0, 360.0) - 180.0;
            float w = clamp(d >= 0.0 ? 1.0 - d / dNext : 1.0 + d / dPrev, 0.0, 1.0);
            w = w * w * (3.0 - 2.0 * w);          // smooth partition of unity
            dh += w * u[i * 3 + 0];
            ds += w * u[i * 3 + 1];
            dl += w * u[i * 3 + 2];
        }
        dh /= 100.0; ds /= 100.0; dl /= 100.0;
        // neutral / near-neutral pixels are left alone (no noise amplification)
        float cw = smoothstep(0.012, 0.07, lch.y);
        lch.z += dh * 0.5236 * cw;                 // ±30°
        lch.y *= max(0.0, 1.0 + ds * cw);
        if (dl < 0.0) lch.x *= 1.0 + dl * 0.55 * cw;
        else lch.x += (1.0 - lch.x) * dl * 0.45 * cw;
        imageStore(o, ivec2(gid), vec4(oklch2srgb(lch), src.a));
}
