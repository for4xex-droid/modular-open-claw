#version 300 es
precision highp float;

in vec2 v_uv;
in vec2 v_state; // x: active, y: morphology
in vec4 v_elements; // x: C, y: N, z: P, w: H
in vec4 v_elements_extra; // x: O, y: S, z: Fe, w: Si

out vec4 fragColor;

// テーマカラー uniform (cssVarブリッジから渡される)
uniform vec3 u_primary_color;
uniform vec3 u_secondary_color;

void main() {
    if (v_state.x < 0.5) {
        // 非アクティブセルは透明
        discard;
    }

    // 各元素の比率をカラーにする
    float total = v_elements.x + v_elements.y + v_elements.z + v_elements.w +
                  v_elements_extra.x + v_elements_extra.y + v_elements_extra.z + v_elements_extra.w;
    
    vec3 color = vec3(0.1, 0.1, 0.1); // ベース

    if (total > 0.0) {
        float c = v_elements.x / total;
        float n = v_elements.y / total;
        float p = v_elements.z / total;
        float h = v_elements.w / total;
        
        // 元素カラーマッピング
        // 炭素(グリーン)、窒素(ブルー)、リン(イエロー/オレンジ)
        color = vec3(c + p * 0.8, n + c * 0.2 + p * 0.8, n * 0.8 + h * 0.5);
    }

    // 形態に応じた形状 procedural 描画 (SDF)
    float dist = distance(v_uv, vec2(0.5));
    float alpha = 1.0;

    int morph = int(v_state.y);
    if (morph == 1) { // Producer (丸っこい)
        alpha = smoothstep(0.48, 0.45, dist);
        color += vec3(0.0, 0.3, 0.0); // 緑強め
    } else if (morph == 3) { // Predator (トゲトゲ)
        // トゲトゲのSDF
        float angle = atan(v_uv.y - 0.5, v_uv.x - 0.5);
        float spikes = 0.5 + 0.15 * sin(angle * 8.0);
        alpha = smoothstep(spikes, spikes - 0.03, dist);
        color += vec3(0.5, 0.0, 0.0); // 赤強め
    } else { // Basic / Consumer / Decomposer
        float round_box = length(max(abs(v_uv - 0.5) - 0.4, 0.0));
        alpha = smoothstep(0.08, 0.05, round_box);
    }

    fragColor = vec4(color, alpha);
}
