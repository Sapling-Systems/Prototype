#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform float renderWidth;
uniform float blurRadius;

out vec4 finalColor;

vec4 premultiply(vec4 c) { c.rgb *= c.a; return c; }
vec4 unpremultiply(vec4 c) { if (c.a > 1e-6) c.rgb /= c.a; else c.rgb = vec3(0.0); return c; }

void main()
{
    vec4 modulate = fragColor * colDiffuse;

    float r = clamp(blurRadius, 0.0, 20.0);   // IMPORTANT: cap for performance in single-pass 2D
    if (r < 0.5)
    {
        finalColor = texture(texture0, fragTexCoord) * modulate;
        return;
    }

    ivec2 ts = textureSize(texture0, 0);
    vec2 texel = 1.0 / vec2(max(ts.x, 1), max(ts.y, 1));

    // Gaussian sigma heuristic
    float sigma = max(r * 0.5, 0.001);
    float invTwoSigma2 = 1.0 / (2.0 * sigma * sigma);

    // Convert radius to integer taps and cap hard
    int R = int(ceil(r));
    R = clamp(R, 1, 20); // (2R+1)^2 => up to 41*41=1681 samples worst-case; keep this low!

    vec4 accum = vec4(0.0);
    float wsum = 0.0;

    for (int y = -20; y <= 20; y++)
    {
        if (abs(y) > R) continue;
        for (int x = -20; x <= 20; x++)
        {
            if (abs(x) > R) continue;

            float d2 = float(x*x + y*y);
            float w = exp(-d2 * invTwoSigma2);

            vec4 s = premultiply(texture(texture0, fragTexCoord + vec2(x, y) * texel));

            // alpha-weighting to reduce hidden-RGB halos
            float aw = clamp(s.a, 0.0, 1.0);
            float wa = w * mix(0.25, 1.0, aw);

            accum += s * wa;
            wsum  += wa;
        }
    }

    vec4 blurred = (wsum > 1e-6) ? (accum / wsum) : vec4(0.0);
    blurred = unpremultiply(blurred);

    finalColor = blurred * modulate;
}
