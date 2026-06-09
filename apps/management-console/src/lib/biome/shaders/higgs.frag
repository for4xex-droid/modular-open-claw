#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_scene_tex;
uniform vec2 u_impact_center; // 0..1
uniform float u_time;
uniform float u_intensity; // エフェクト強度

void main() {
    vec2 to_center = v_uv - u_impact_center;
    float dist = length(to_center);

    // 重力レンズ歪みの計算 (アインシュタインリング模倣)
    vec2 offset = vec2(0.0);
    if (dist > 0.01 && dist < 0.3) {
        float force = u_intensity * 0.05 / (dist + 0.05);
        // 波紋のような動きを少し加える
        force *= (1.0 + 0.1 * sin(dist * 50.0 - u_time * 10.0));
        offset = normalize(to_center) * force;
    }

    // 歪ませた座標からシーンテクスチャをサンプリング
    // 屈折の色収差 (Chromatic Aberration) の表現のために RGB チャンネルをずらす
    vec4 r_col = texture(u_scene_tex, v_uv - offset);
    vec4 g_col = texture(u_scene_tex, v_uv - offset * 1.05);
    vec4 b_col = texture(u_scene_tex, v_uv - offset * 1.1);

    fragColor = vec4(r_col.r, g_col.g, b_col.b, 1.0);
}
