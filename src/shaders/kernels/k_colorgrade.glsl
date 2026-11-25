// Colour grading (Lightroom-style): shadows / midtones / highlights / global, each with a hue +
// saturation push and a luminance shift, done in Oklab so colours move evenly.
// u: [zone*3 + 0] hue rad, [+1] sat 0..1, [+2] lum -1..1 for zones 0 shadows, 1 mids, 2 highs, 3 global
//    12 blending 0..1, 13 balance -1..1
layout(set = 0, binding = 7) uniform texture2D a;
vec2 push(int z) { float h = u[z * 3], s = u[z * 3 + 1]; return vec2(cos(h), sin(h)) * s * 0.19; }
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 lab = lin2oklab(toLinear(clamp(s.rgb, 0.0, 1.0)));
    float L = clamp(lab.x, 0.0, 1.0);
    float b = u[13] * 0.22;
    float bw = mix(0.06, 0.34, clamp(u[12], 0.0, 1.0));
    float ws = 1.0 - smoothstep(0.36 + b - bw, 0.36 + b + bw, L);
    float wh = smoothstep(0.68 + b - bw, 0.68 + b + bw, L);
    float wm = clamp(1.0 - ws - wh, 0.0, 1.0);
    // chroma pushes fade out at pure black / white so they never look like a coloured fog
    float ends = smoothstep(0.0, 0.08, L) * (1.0 - smoothstep(0.96, 1.0, L));
    lab.yz += (push(0) * ws + push(1) * wm + push(2) * wh + push(3)) * ends;
    float dl = u[2] * ws + u[5] * wm + u[8] * wh + u[11];
    lab.x += dl * 0.18 * (dl > 0.0 ? (1.0 - L) : L) * 2.0;
    vec3 c = toSRGB(max(oklab2lin(lab), vec3(0.0)));
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
