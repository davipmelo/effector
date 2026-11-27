// Image Overlay (0.9.1): places an image over the frame; straight alpha out (the layer blend mixes it).
// u: 0 has image, 1 fit (0 fill, 1 fit, 2 stretch, 3 tile), 2 scale, 3 posX, 4 posY (-1..1), 5 rotation (rad),
//    6 flipH, 7 flipV, 8 texture aspect, 9 perspective H, 10 perspective V, 11 skew X, 12 skew Y, 13 bulge,
//    14 wave amount, 15 wave cycles, 16 invert, 17 black, 18 white, 19 gamma, 20 saturation,
//    21..23 tint rgb, 24 tint amount, 25 edge softness (px)
// textures: img=1 (the output has the size of the layer input)
layout(set = 0, binding = 7) uniform texture2D img;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    ivec2 sz = imageSize(o);
    if (gid.x >= uint(sz.x) || gid.y >= uint(sz.y)) return;
    if (u[0] < 0.5) { imageStore(o, ivec2(gid), vec4(0.0)); return; }
    vec2 S = vec2(sz);
    float ia = S.x / S.y;
    // image plane, in units of the image height, centred
    vec2 q = ((vec2(gid) + 0.5) / S - 0.5) * vec2(ia, 1.0);
    vec2 half_ = vec2(ia, 1.0) * 0.5;
    // distortions of the overlay (sliders)
    float rn = length(q) / length(half_);
    if (abs(u[13]) > 1e-4) q *= pow(max(rn, 1e-4), u[13] * 0.9);
    if (u[14] > 1e-4) {
        float f = u[15] * 6.2831853;
        float amp = u[14] * 0.05;
        q += vec2(sin(q.y * f + 1.3), sin(q.x * f + 4.1)) * amp;
    }
    float d = 1.0 + u[9] * (q.x / half_.x) * 0.6 + u[10] * (q.y / half_.y) * 0.6;
    q /= max(d, 0.15);
    q = vec2(q.x + u[11] * 0.6 * q.y, q.y + u[12] * 0.6 * q.x / ia);
    // placement
    q -= vec2(u[3], u[4]) * half_;
    q = rot2(q, -u[5]);
    float sc = max(u[2], 1e-3);
    q /= sc;
    int fit = int(u[1] + 0.5);
    float ta = max(u[8], 1e-4);
    vec2 bs;
    if (fit == 0) {
        // cover the image even when rotated
        float c = abs(cos(u[5])), s = abs(sin(u[5]));
        vec2 need = vec2(ia * c + s, ia * s + c);
        bs = ta > need.x / need.y ? vec2(need.y * ta, need.y) : vec2(need.x, need.x / ta);
    } else if (fit == 1) {
        bs = ta > ia ? vec2(ia, ia / ta) : vec2(ta, 1.0);
    } else if (fit == 2) {
        bs = vec2(ia, 1.0);
    } else {
        bs = vec2(0.5 * ta, 0.5);
    }
    vec2 t = q / bs + 0.5;
    if (u[6] > 0.5) t.x = 1.0 - t.x;
    if (u[7] > 0.5) t.y = 1.0 - t.y;
    vec4 c;
    float cov = 1.0;
    if (fit == 3) {
        c = textureLod(sampler2D(img, sRep), t, 0.0);
    } else if (fit == 0) {
        c = textureLod(sampler2D(img, sMir), t, 0.0);
    } else {
        c = textureLod(sampler2D(img, sLin), clamp(t, 0.0, 1.0), 0.0);
        // soft edge of the placed image, about one output pixel
        vec2 e = min(t, 1.0 - t) * bs * sc * S.y;
        cov = clamp(min(e.x, e.y) / max(u[25], 1.0) + 0.5, 0.0, 1.0);
    }
    vec3 col = clamp(c.rgb, 0.0, 1.0);
    float l = luma(col);
    col = clamp(mix(vec3(l), col, u[20]), 0.0, 1.0);
    col = clamp((col - u[17]) / max(u[18] - u[17], 1e-3), 0.0, 1.0);
    col = pow(col, vec3(1.0 / max(u[19], 0.05)));
    if (u[16] > 0.5) col = 1.0 - col;
    col = mix(col, vec3(luma(col)) * vec3(u[21], u[22], u[23]), u[24]);
    imageStore(o, ivec2(gid), vec4(col, clamp(c.a, 0.0, 1.0) * cov));
}
