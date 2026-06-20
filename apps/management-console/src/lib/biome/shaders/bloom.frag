#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_sceneTex;
uniform sampler2D u_bloomTex;
uniform int u_mode;          // 0: threshold, 1: horizontal blur, 2: vertical blur, 3: composite/blend
uniform vec2 u_resolution;
uniform float u_bloomIntensity;

// Gaussian weights
const float weight[5] = float[](0.2270270270, 0.1945945946, 0.1216216216, 0.0540540541, 0.0162162162);

void main() {
    if (u_mode == 0) {
        // 1. Threshold pass: extract only very bright elements (glow edges etc.)
        vec4 color = texture(u_sceneTex, v_uv);
        float brightness = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
        // 高い閾値で本当に光っている部分のみ抽出（0.35→0.7）
        if (brightness > 0.7) {
            // 閾値を超えた分だけ抽出（ソフトな減衰）
            float excess = (brightness - 0.7) / 0.3;
            fragColor = vec4(color.rgb * clamp(excess, 0.0, 1.0), 1.0);
        } else {
            fragColor = vec4(0.0, 0.0, 0.0, 1.0);
        }
    } else if (u_mode == 1) {
        // 2. Horizontal Gaussian blur
        vec2 tex_offset = 1.0 / u_resolution;
        vec3 result = texture(u_sceneTex, v_uv).rgb * weight[0];
        for (int i = 1; i < 5; ++i) {
            result += texture(u_sceneTex, v_uv + vec2(tex_offset.x * float(i), 0.0)).rgb * weight[i];
            result += texture(u_sceneTex, v_uv - vec2(tex_offset.x * float(i), 0.0)).rgb * weight[i];
        }
        fragColor = vec4(result, 1.0);
    } else if (u_mode == 2) {
        // 3. Vertical Gaussian blur
        vec2 tex_offset = 1.0 / u_resolution;
        vec3 result = texture(u_sceneTex, v_uv).rgb * weight[0];
        for (int i = 1; i < 5; ++i) {
            result += texture(u_sceneTex, v_uv + vec2(0.0, tex_offset.y * float(i))).rgb * weight[i];
            result += texture(u_sceneTex, v_uv - vec2(0.0, tex_offset.y * float(i))).rgb * weight[i];
        }
        fragColor = vec4(result, 1.0);
    } else if (u_mode == 3) {
        // 4. Composite blend pass: 単純な加算合成（トーンマッピングなし）
        vec3 sceneColor = texture(u_sceneTex, v_uv).rgb;
        vec3 bloomColor = texture(u_bloomTex, v_uv).rgb;
        // 加算合成のみ。HDRトーンマッピングは暗部を持ち上げるため使用しない
        vec3 result = sceneColor + bloomColor * u_bloomIntensity;
        fragColor = vec4(clamp(result, 0.0, 1.0), 1.0);
    } else {
        fragColor = texture(u_sceneTex, v_uv);
    }
}
