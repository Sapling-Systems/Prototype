#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform float renderWidth;
uniform float blurRadius;

out vec4 finalColor;

// Gaussian weight function
float gaussian(float x, float sigma) {
    return exp(-(x * x) / (2.0 * sigma * sigma));
}

void main()
{
    // Early exit for no blur
    if (blurRadius <= 0.001) {
        finalColor = texture(texture0, fragTexCoord);
        return;
    }

    // Calculate sigma based on blur radius
    float sigma = max(blurRadius * 0.3, 0.0001);

    // Kernel size - scaled based on blur radius
    int kernelSize = int(ceil(blurRadius));
    kernelSize = max(1, min(kernelSize, 12));

    vec4 colorSum = vec4(0.0);
    float weightSum = 0.0;

    // Calculate texel size (assuming square aspect ratio)
    float texelSize = 1.0 / renderWidth;

    // Apply 2D Gaussian blur with premultiplied alpha
    for (int y = -kernelSize; y <= kernelSize; y++) {
        for (int x = -kernelSize; x <= kernelSize; x++) {
            vec2 offset = vec2(float(x), float(y));
            float distance = length(offset);
            float weight = gaussian(distance, sigma);

            vec4 sampleColor = texture(texture0, fragTexCoord + offset * texelSize);

            // Premultiply alpha to avoid white halos on transparent edges
            colorSum.rgb += sampleColor.rgb * sampleColor.a * weight;
            colorSum.a += sampleColor.a * weight;
            weightSum += weight;
        }
    }

    // Normalize and unpremultiply alpha
    finalColor.rgb = colorSum.a > 0.0001 ? colorSum.rgb / colorSum.a : vec3(0.0);
    finalColor.a = colorSum.a / weightSum;
}
