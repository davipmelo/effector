// Canvas display: composites the rendered image over the checkerboard (replaces the Metal
// vertex/fragment display pass). Output is the view-sized region of the image.
// u: imgX, imgY, imgW, imgH (output pixels; image rect inside the output), checker size, nearest
layout(set = 0, binding = 7) uniform texture2D t;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 p = vec2(gid) + 0.5;
    vec2 uv = (p - vec2(u[0], u[1])) / vec2(u[2], u[3]);
    vec4 c = u[5] > 0.5 ? textureLod(sampler2D(t, sNear), uv, 0.0) : textureLod(sampler2D(t, sLin), uv, 0.0);
    vec2 cell = floor(p / u[4]);
    float ch = fmodc(cell.x + cell.y, 2.0) < 1.0 ? 0.28 : 0.36;
    vec3 col = mix(vec3(ch), clamp(c.rgb, 0.0, 1.0), clamp(c.a, 0.0, 1.0));
    // egui samples textures as sRGB-gamma values: store them as-is.
    imageStore(o, ivec2(gid), vec4(col, 1.0));
}
