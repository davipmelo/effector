// Separable Lanczos-3 resampling pass (replaces MPSImageLanczosScale). Widens the kernel when
// downscaling so it filters properly. u: axis (0 = x, 1 = y)
layout(set = 0, binding = 7) uniform texture2D a;
float sinc(float x) { if (abs(x) < 1e-5) return 1.0; float px = 3.14159265 * x; return sin(px) / px; }
float lanczos3(float x) { return abs(x) >= 3.0 ? 0.0 : sinc(x) * sinc(x / 3.0); }
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    ivec2 osz = imageSize(o);
    if (gid.x >= uint(osz.x) || gid.y >= uint(osz.y)) return;
    ivec2 isz = textureSize(sampler2D(a, sNear), 0);
    int axis = int(u[0]);
    float inN = float(axis == 0 ? isz.x : isz.y);
    float outN = float(axis == 0 ? osz.x : osz.y);
    float scale = inN / outN;
    float support = max(1.0, scale);
    float center = ((axis == 0 ? float(gid.x) : float(gid.y)) + 0.5) * scale;
    int lo = int(floor(center - 3.0 * support));
    int hi = int(ceil(center + 3.0 * support));
    vec4 acc = vec4(0.0);
    float wsum = 0.0;
    for (int i = lo; i <= hi; i++) {
        float w = lanczos3((float(i) + 0.5 - center) / support);
        if (w == 0.0) continue;
        int ci = clamp(i, 0, int(inN) - 1);
        ivec2 q = axis == 0 ? ivec2(ci, int(gid.y)) : ivec2(int(gid.x), ci);
        acc += texelFetch(sampler2D(a, sNear), q, 0) * w;
        wsum += w;
    }
    imageStore(o, ivec2(gid), acc / max(wsum, 1e-6));
}
