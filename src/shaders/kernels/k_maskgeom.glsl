// Gradient masks (1 = selected), computed in image space so they follow crops.
// u: 0 kind (0 linear, 1 radial), 1-4 crop x, y, w, h, 5-6 image size / long side (ax, ay),
//    linear: 7 x, 8 y, 9 angle deg, 10 width
//    radial: 7 x, 8 y, 9 rx, 10 ry, 11 angle deg, 12 feather
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec2 a = vec2(u[5], u[6]);
    vec2 p = (vec2(u[1], u[2]) + uv * vec2(u[3], u[4])) * a;   // long-side units
    vec2 c = vec2(u[7], u[8]) * a;
    float v;
    if (u[0] < 0.5) {
        float ang = u[9] * 0.0174533;
        vec2 dir = vec2(cos(ang), sin(ang));
        float t = dot(p - c, dir) / max(u[10], 1e-4) + 0.5;
        v = 1.0 - smoothstep(0.0, 1.0, t);
    } else {
        float ang = -u[11] * 0.0174533;
        vec2 d = p - c;
        vec2 q = vec2(cos(ang) * d.x - sin(ang) * d.y, sin(ang) * d.x + cos(ang) * d.y);
        float r = length(q / max(vec2(u[9], u[10]), vec2(1e-4)));
        float f = clamp(u[12], 0.0, 1.0);
        v = 1.0 - smoothstep(1.0 - f - 1e-3, 1.0, r);
    }
    imageStore(o, ivec2(gid), vec4(v, v, v, 1.0));
}
