#version 330

// Input vertex attributes (from vertex shader)
in vec2 fragTexCoord;
in vec4 fragColor;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;

// Output fragment color
out vec4 finalColor;

void main()
{
    // Texel color fetching from texture sampler
    // NOTE: Calculate alpha using signed distance field (SDF)
    vec4 texel = texture(texture0, fragTexCoord);

    // Raylib SDF stores distance in alpha, centered at 0.5
    float distance = texel.a - 0.5;

    // Calculate adaptive smoothing based on screen-space derivatives
    // This gives better anti-aliasing at different scales
    float smoothing = length(vec2(dFdx(distance), dFdy(distance)));

    // Clamp smoothing to avoid artifacts at very small/large scales
    smoothing = clamp(smoothing, 0.0001, 0.25);

    // Use smoothstep for anti-aliased edges
    float alpha = smoothstep(-smoothing, smoothing, distance);

    // Calculate final fragment color
    finalColor = vec4(fragColor.rgb, fragColor.a * alpha);
}
