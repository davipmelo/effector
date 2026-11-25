// Half prism / split filter: one side of a line shows a refracted (shifted, optionally mirrored and
// zoomed) copy of the image, with a rainbow seam along the cut.
// u: 0 angle deg, 1 position (-1..1), 2 shift px, 3 mirror, 4 zoom, 5 seam width, 6 seam rainbow,
//    7 dispersion, 8 amount, 9 feather
// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    float an = u[0] * 0.0174533;
    vec2 nrm = vec2(cos(an), -sin(an));                                 // normal of the cut line
    float asp = size.x / size.y;
    vec2 p = (uv - 0.5) * vec2(asp, 1.0);
    float side = dot(p, nrm) - u[1] * 0.5;                              // >0 = refracted half
    vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float fe = max(u[9], 0.001);
    float w = smoothstep(-fe, fe, side);
    if (w <= 0.0) { imageStore(o, ivec2(gid), base); return; }
    // refracted coordinates: mirror across the line, zoom around the line, shift along the normal
    vec2 q = p;
    if (u[3] > 0.5) q = p - 2.0 * side * nrm;
    vec2 onLine = p - side * nrm;
    q = onLine + (q - onLine) / max(u[4], 0.05);
    q += nrm * u[2] / size.y;
    vec2 ruv = q / vec2(asp, 1.0) + 0.5;
    float disp = u[7] * 0.02 * (1.0 + 4.0 * exp(-abs(side) / max(u[5], 0.002)));
    vec3 g = vec3(textureLod(sampler2D(soft, sMir), ruv + nrm * disp, 0.0).r,
                  textureLod(sampler2D(soft, sMir), ruv, 0.0).g,
                  textureLod(sampler2D(soft, sMir), ruv - nrm * disp, 0.0).b);
    // rainbow band along the seam
    float s = abs(side) / max(u[5], 0.002);
    vec3 rainbow = 0.5 + 0.5 * cos(6.2831853 * (vec3(0.0, 0.33, 0.67) + side / max(u[5], 0.002) * 0.5));
    g = mix(g, g + rainbow * 0.45, u[6] * exp(-s * s));
    vec3 c = mix(base.rgb, g, clamp(u[8], 0.0, 1.0) * w);
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), base.a));
}
