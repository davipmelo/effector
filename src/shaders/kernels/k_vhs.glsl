// textures: a=1 orig=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D orig;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 p = vec2(gid) + 0.5;
        vec2 uv = p / size;
        float unit = u[1], seed = u[0];
        float lineH = max(1.0, u[28]);
        float line = floor(p.y / lineH);
        float yn = p.y / size.y;

        // --- luma: band-limited + VCR edge enhancement (ringing halos)
        float lumaR = max(0.5, u[2] * unit);
        float accN = 0.0, accW = 0.0, wN = 0.0, wW = 0.0;
        for (int i = -6; i <= 6; i++) {
            float t = float(i) / 6.0;
            float g = exp(-t * t * 2.2);
            accN += rgb2yiq(textureLod(sampler2D(a, sLin), uv + vec2(t * lumaR, 0.0) / size, 0.0).rgb).x * g; wN += g;
            accW += rgb2yiq(textureLod(sampler2D(a, sLin), uv + vec2(t * lumaR * 3.2, 0.0) / size, 0.0).rgb).x * g; wW += g;
        }
        float Y = accN / wN;
        float Yw = accW / wW;
        Y += (Y - Yw) * u[3] * 1.6;

        // --- chroma: very low bandwidth, smeared right, delayed, line-averaged vertically
        vec2 IQ;
        if (u[4] > 0.5) {
            float chromaR = max(1.0, u[5] * unit);
            float shift = u[6] * unit;
            vec2 acc = vec2(0.0); float wc = 0.0;
            for (int i = 0; i <= 12; i++) {
                float t = float(i) / 12.0;
                float w = exp(-t * 2.5);
                vec2 o1 = vec2(-t * chromaR - shift, 0.0) / size;
                vec2 o2 = vec2(-t * chromaR - shift, -lineH) / size;
                acc += (rgb2yiq(textureLod(sampler2D(a, sLin), uv + o1, 0.0).rgb).yz + rgb2yiq(textureLod(sampler2D(a, sLin), uv + o2, 0.0).rgb).yz) * 0.5 * w;
                wc += w;
            }
            IQ = acc / wc;
        } else {
            IQ = rgb2yiq(textureLod(sampler2D(a, sLin), uv, 0.0).rgb).yz;
        }
        // chroma noise: horizontal coloured streaks
        if (u[7] > 0.0) {
            float sx = floor(p.x / max(1.0, unit * 6.0));
            float n1 = vnoise(vec2(sx * 0.35, line), seed + 41.0) - 0.5;
            float n2 = vnoise(vec2(sx * 0.35, line), seed + 77.0) - 0.5;
            IQ += vec2(n1, n2) * u[7] * 0.22 * (1.0 - smoothstep(0.6, 1.0, Y));
        }
        IQ *= u[8];
        vec3 c = yiq2rgb(vec3(Y, IQ));

        // --- tape noise: snow elongated along the scan direction
        if (u[9] > 0.5) {
            float n = gnoise(vec2(floor(p.x / max(1.0, unit * 2.5)), line), seed + 1.0);
            float ln = hash12(vec2(line, seed + 9.0)) - 0.5;
            c += (n * 0.055 + ln * 0.035) * u[10];
            float streak = step(0.993 - u[10] * 0.012, hash12(vec2(line * 1.7, seed + 4.0)));
            float sx = hash12(vec2(line, seed + 5.0)) * size.x;
            float len = (hash12(vec2(line, seed + 6.0)) * 0.25 + 0.04) * size.x;
            if (streak > 0.5 && p.x > sx && p.x < sx + len) c += 0.4 * u[10] * (1.0 - (p.x - sx) / len);
        }
        // tracking band noise
        if (u[22] > 0.5) {
            float d = abs(yn - u[23]) / max(0.004, u[24]);
            float band = smoothstep(1.0, 0.0, d);
            if (band > 0.0) {
                float n = hash13(vec3(floor(p.x / max(1.0, unit * 2.0)), line, seed + 2.0));
                float lines = step(0.55, hash12(vec2(line, seed + 31.0)));
                c = mix(c, vec3(n * 0.9 + 0.1), band * lines * 0.55 * min(1.0, u[25] / 8.0));
            }
        }
        // head-switching noise at the bottom
        if (u[26] > 0.5) {
            float h0 = 1.0 - u[27];
            float hs = smoothstep(h0, h0 + 0.004, yn);
            if (hs > 0.0) {
                float n = hash13(vec3(floor(p.x / max(1.0, unit * 3.0)), line, seed + 17.0));
                c = mix(c, vec3(n), hs * 0.25);
                c *= 1.0 - hs * 0.25;
            }
        }
        // dropouts: oxide loss = bright horizontal flecks
        if (u[11] > 0.5) {
            float r = hash12(vec2(line, seed + 11.0));
            if (r < 0.025 * u[12]) {
                float sx = hash12(vec2(line, seed + 12.0)) * size.x;
                float len = (hash12(vec2(line, seed + 13.0)) * 0.05 + 0.006) * size.x;
                float t = (p.x - sx) / len;
                if (t > 0.0 && t < 1.0) c = mix(c, vec3(0.97), 0.9 * (1.0 - t * t));
            }
        }

        // --- tone: lifted blacks, soft highlight roll-off, colour cast of the tape
        float k = u[29];
        c = (c - 0.5) * k + 0.5;
        float roll = u[19];
        c = mix(c, (1.0 - exp(-max(c, 0.0) * 2.0)) / (1.0 - exp(-2.0)), roll * 0.6);
        c = u[18] + c * (1.0 - u[18]);
        float w = u[20], t = u[21];
        c *= vec3(1.0 + 0.07 * w, 1.0 - 0.07 * t * 0.6, 1.0 - 0.09 * w);
        c *= vec3(1.0 + 0.05 * t, 1.0 - 0.06 * t, 1.0 + 0.05 * t);

        // --- interlaced scanlines
        if (u[13] > 0.5) {
            float sp = max(2.0, u[15]);
            float ph = fract((p.y - 0.5) / sp);
            float s = smoothstep(0.35, 0.65, abs(ph - 0.5) * 2.0);
            c *= 1.0 - u[14] * s;
        }
        if (u[16] > 0.5) {
            vec2 q = uv - 0.5;
            c *= 1.0 - u[17] * smoothstep(0.2, 0.8, length(q * vec2(1.0, 0.85)));
        }
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), texelFetch(sampler2D(orig, sNear), ivec2(gid), 0).a));
}
