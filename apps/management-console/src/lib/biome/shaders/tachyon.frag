#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_current_tex;
uniform sampler2D u_history_tex;
uniform float u_blend_factor; // 過去と現在のブレンド比率 (残像の長さ)

void main() {
    vec4 current = texture(u_current_tex, v_uv);
    vec4 history = texture(u_history_tex, v_uv);

    // タキオンの残像（タイムトレイル）エフェクト
    // シアン（青緑）寄りのカラーフィードバックをノイズと共にブレンド
    vec3 ghost_color = history.rgb * vec3(0.6, 0.95, 1.0); // シアンがかった色調
    
    vec3 final_rgb = mix(current.rgb, ghost_color, u_blend_factor);
    fragColor = vec4(final_rgb, 1.0);
}
