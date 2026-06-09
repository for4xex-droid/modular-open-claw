#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec2 a_cell_pos; // 0..127
layout(location = 3) in vec2 a_state;    // x: active, y: morphology
layout(location = 4) in vec4 a_elements; // x: C, y: N, z: P, w: H
layout(location = 5) in vec4 a_elements_extra; // x: O, y: S, z: Fe, w: Si

out vec2 v_uv;
out vec2 v_state;
out vec4 v_elements;
out vec4 v_elements_extra;

uniform vec2 u_grid_size; // 128, 128

void main() {
    v_uv = a_uv;
    v_state = a_state;
    v_elements = a_elements;
    v_elements_extra = a_elements_extra;

    // セルの正規化座標 (0..1)
    vec2 norm_cell_pos = a_cell_pos / u_grid_size;
    // セル自体のサイズ
    vec2 cell_size = 2.0 / u_grid_size;

    // 頂点の位置をセルの位置に合わせてオフセット & スケール
    // 画面全体が -1..1 の座標系
    vec2 final_pos = -1.0 + norm_cell_pos * 2.0 + (a_pos + 1.0) * 0.5 * cell_size;
    gl_Position = vec4(final_pos, 0.0, 1.0);
}
