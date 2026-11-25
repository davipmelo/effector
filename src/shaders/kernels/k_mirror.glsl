// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        int mode = int(u[0]);
        float c = u[2];
        if (mode == 0) uv.x = uv.x > c ? 2.0 * c - uv.x : uv.x;           // left half reflected to the right
        else if (mode == 1) uv.x = uv.x < c ? 2.0 * c - uv.x : uv.x;      // right half reflected to the left
        else if (mode == 2) uv.y = uv.y > c ? 2.0 * c - uv.y : uv.y;      // top reflected down
        else if (mode == 3) uv.y = uv.y < c ? 2.0 * c - uv.y : uv.y;      // bottom reflected up
        else if (mode == 4) uv = 0.5 - abs(uv - 0.5);                       // four-way
        else {
            float n = max(1.0, floor(u[1]));
            vec2 t = uv * n;
            vec2 f = fract(t);
            vec2 k = floor(t);
            if (mode == 5) f = mix(f, 1.0 - f, greaterThan(fmodc(k, 2.0), vec2(0.5)));       // mirrored tiles
            uv = f;
        }
        imageStore(o, ivec2(gid), vec4(textureLod(sampler2D(a, sMir), uv, 0.0).rgb, texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
