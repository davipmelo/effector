// Separable gaussian blur pass (replaces MPSImageGaussianBlur, edge mode clamp).
// u: sigma, dirX, dirY
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    ivec2 size = imageSize(o);
    if (gid.x >= uint(size.x) || gid.y >= uint(size.y)) return;
    float sigma = max(u[0], 0.01);
    ivec2 dir = ivec2(int(u[1]), int(u[2]));
    int r = int(ceil(sigma * 3.0));
    ivec2 sz = textureSize(sampler2D(a, sNear), 0);
    ivec2 p = ivec2(gid);
    float inv = -0.5 / (sigma * sigma);
    vec4 acc = vec4(0.0);
    float wsum = 0.0;
    for (int i = -r; i <= r; i++) {
        float w = exp(float(i * i) * inv);
        ivec2 q = clamp(p + dir * i, ivec2(0), sz - 1);
        acc += texelFetch(sampler2D(a, sNear), q, 0) * w;
        wsum += w;
    }
    imageStore(o, p, acc / wsum);
}
