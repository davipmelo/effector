// Invert. u: 0 mode (0 all, 1 luminance only, 2 colours only, 3 red, 4 green, 5 blue), 1 amount (0..1;
// all-zero uniforms = legacy full invert)
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    int mode = int(u[0]);
    float amt = u[1] <= 0.0 && u[0] <= 0.0 ? 1.0 : u[1];
    vec3 r = 1.0 - c.rgb;
    if (mode == 1) {
        vec3 lab = lin2oklab(toLinear(clamp(c.rgb, 0.0, 1.0)));
        lab.x = 1.0 - lab.x;
        r = toSRGB(max(oklab2lin(lab), vec3(0.0)));
    } else if (mode == 2) {
        vec3 lab = lin2oklab(toLinear(clamp(c.rgb, 0.0, 1.0)));
        lab.yz = -lab.yz;
        r = toSRGB(max(oklab2lin(lab), vec3(0.0)));
    } else if (mode >= 3) {
        r = c.rgb;
        r[mode - 3] = 1.0 - c.rgb[mode - 3];
    }
    imageStore(o, ivec2(gid), vec4(clamp(mix(c.rgb, r, amt), 0.0, 1.0), c.a));
}
