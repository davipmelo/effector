import Foundation
import Metal

enum CanvasShaders {
    static let source = """
    // Cover-fit sample of the image into a texture of another aspect (for blurred backgrounds).
    kernel void k_cover(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 os = OSIZE;
        float2 is = float2(a.get_width(), a.get_height());
        float s = max(os.x / is.x, os.y / is.y);
        float2 p = (float2(gid) + 0.5 - os * 0.5) / s + is * 0.5;
        o.write(float4(a.sample(sLin, p / is).rgb, 1.0), gid);
    }

    // Final canvas. u: imageRect x,y,w,h (px), radius(px), fill (0 color,1 texture,2 gradient), color rgb (6..8),
    //    color2 rgb (9..11), shadow, shadow size(px), bg dim
    kernel void k_canvas(KIN, texture2d<float> img [[texture(1)]], texture2d<float> bg [[texture(2)]]) {
        GUARD
        float2 p = float2(gid) + 0.5;
        float2 os = OSIZE;
        float2 r0 = float2(u[0], u[1]), rs = float2(u[2], u[3]);
        float rad = u[4];
        float2 c = r0 + rs * 0.5;
        float2 q = abs(p - c) - (rs * 0.5 - rad);
        float sd = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - rad;
        float3 back;
        int fill = int(u[5]);
        if (fill == 1) back = bg.sample(sLin, p / os).rgb * (1.0 - u[14]);
        else if (fill == 2) back = mix(float3(u[6], u[7], u[8]), float3(u[9], u[10], u[11]), p.y / os.y);
        else back = float3(u[6], u[7], u[8]);
        // soft drop shadow under the photo
        float sh = u[12] * 0.6 * exp(-max(sd - u[13] * 0.15, 0.0) / max(1.0, u[13]));
        back *= 1.0 - sh;
        float inside = 1.0 - smoothstep(-0.75, 0.75, sd);
        float2 iuv = (p - r0) / rs;
        float4 s = img.sample(sLin, iuv);
        float3 col = mix(back, s.rgb, inside * s.a);
        o.write(float4(col, 1.0), gid);
    }
    """
}

enum CanvasCompose {
    /// Places `image` (already processed) on a canvas. `unit` = render pixels per original pixel.
    static func apply(_ image: MTLTexture, _ spec: CanvasSpec, _ ctx: RenderContext) -> MTLTexture {
        let (size, rect) = spec.layout(Double(image.width), Double(image.height))
        let W = max(1, Int(size.width)), H = max(1, Int(size.height))
        var bg = image
        if spec.fill == "blur" {
            let f = 6
            let small = ctx.apply("k_cover", [image], [], w: max(1, W / f), h: max(1, H / f))
            bg = ctx.blur(small, sigma: max(0.5, spec.blur / 100 * Double(max(W, H)) / Double(f) * 0.04))
        }
        let fill: Float = ["blur": 1, "gradient": 2][spec.fill] ?? 0
        let radius = Float(spec.radius / 100 * Double(min(rect.width, rect.height)) * 0.5)
        let shadowSize = Float(Double(max(W, H)) * 0.025)
        var u: [Float] = [Float(rect.minX), Float(rect.minY), Float(rect.width), Float(rect.height), radius, fill]
        u += spec.color.floats + spec.color2.floats
        u += [Float(spec.shadow / 100), shadowSize, spec.fill == "blur" ? 0.12 : 0]
        return ctx.apply("k_canvas", [image, bg], u, w: W, h: H)
    }
}
