#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_gridTex0; // r: x, g: y, b: active, a: morphology
uniform sampler2D u_gridTex1; // r: C, g: N, b: P, a: H
uniform sampler2D u_gridTex2; // r: O, g: S, b: Fe, a: Si
uniform vec2 u_grid_size;     // 128, 128

uniform vec3 u_primary_color;
uniform vec3 u_secondary_color;
uniform float u_time;
uniform vec2 u_hover_cell;

uniform vec4 u_injection_centers[4];
uniform int u_injection_count;

// HSL to RGB helper
vec3 hsl2rgb(float h, float s, float l) {
    vec3 rgb = clamp(abs(mod(h*6.0+vec3(0.0,4.0,2.0),6.0)-3.0)-1.0, 0.0, 1.0);
    return l + s * (rgb - 0.5) * (1.0 - abs(2.0*l - 1.0));
}

// 8元素比率からHSLを導出してRGBカラーを返す
vec3 getElementColor(ivec2 coord) {
    vec4 e1 = texelFetch(u_gridTex1, coord, 0); // C, N, P, H
    vec4 e2 = texelFetch(u_gridTex2, coord, 0); // O, S, Fe, Si
    
    float C = e1.r, N = e1.g, P = e1.b, H = e1.a;
    float O = e2.r, S = e2.g, Fe = e2.b, Si = e2.a;
    
    float total = C + N + P + H + O + S + Fe + Si;
    if (total < 1.0) {
        return vec3(0.012, 0.016, 0.026);
    }
    
    float c = C / total;
    float n = N / total;
    float p = P / total;
    float h = H / total;
    float o = O / total;
    float s = S / total;
    float fe = Fe / total;
    float si = Si / total;
    
    // 円周平均による色相(Hue)抽出
    float PI = 3.14159265;
    float vx = c * cos(0.0)
             + n * cos(PI * 0.25)
             + p * cos(PI * 0.5)
             + h * cos(PI * 0.75)
             + o * cos(PI)
             + s * cos(PI * 1.25)
             + fe * cos(PI * 1.5)
             + si * cos(PI * 1.75);
             
    float vy = c * sin(0.0)
             + n * sin(PI * 0.25)
             + p * sin(PI * 0.5)
             + h * sin(PI * 0.75)
             + o * sin(PI)
             + s * sin(PI * 1.25)
             + fe * sin(PI * 1.5)
             + si * sin(PI * 1.75);
             
    float hue = atan(vy, vx) / (2.0 * PI);
    if (hue < 0.0) hue += 1.0;
    
    float len = length(vec2(vx, vy));
    float sat = clamp(len * 0.8 + 0.2, 0.0, 1.0);
    
    // 明度は総量に応じて動的変化 (美しく光るレンジに収める)
    float l = clamp(total / 120000.0, 0.4, 0.7);
    
    return hsl2rgb(hue, sat, l);
}

// セルの総エネルギー（元素総量）を返す
float getEnergy(ivec2 coord) {
    if (coord.x < 0 || coord.x >= int(u_grid_size.x) || coord.y < 0 || coord.y >= int(u_grid_size.y)) {
        return 0.0;
    }
    vec4 e1 = texelFetch(u_gridTex1, coord, 0);
    vec4 e2 = texelFetch(u_gridTex2, coord, 0);
    return e1.r + e1.g + e1.b + e1.a + e2.r + e2.g + e2.b + e2.a;
}

// 任意のUV座標でのメタボール場算出（法線計算用）
float getFieldAtUV(vec2 uv) {
    ivec2 cc = ivec2(floor(uv * u_grid_size));
    vec2 cu = fract(uv * u_grid_size);
    float f = 0.0;
    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            ivec2 nc = cc + ivec2(dx, dy);
            if (nc.x >= 0 && nc.x < int(u_grid_size.x) && nc.y >= 0 && nc.y < int(u_grid_size.y)) {
                vec4 nd = texelFetch(u_gridTex0, nc, 0);
                if (nd.b > 0.5) {
                    vec2 center = vec2(dx, dy) + vec2(0.5) - cu;
                    float d = length(center);
                    f += 1.0 / (d * d + 0.15);
                }
            }
        }
    }
    return f;
}

// 擬似乱数ハッシュ (ボロノイノイズ用)
vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return fract(sin(p) * 43758.5453);
}

// 2Dボロノイノイズによる有機的背景
float cellularNoise(vec2 x) {
    vec2 n = floor(x);
    vec2 f = fract(x);
    float m_dist = 8.0;
    for (int j = -1; j <= 1; j++) {
        for (int i = -1; i <= 1; i++) {
            vec2 g = vec2(float(i), float(j));
            vec2 o = hash2(n + g);
            // 時間でゆるやかに呼吸するように波打たせる
            o = 0.5 + 0.5 * sin(u_time * 0.4 + 6.2831 * o);
            vec2 r = g + o - f;
            float d = dot(r, r);
            if (d < m_dist) {
                m_dist = d;
            }
        }
    }
    return sqrt(m_dist);
}

// 注入リップルエフェクト計算
float injectionRipple(vec2 cellPos, vec2 center, float age, float elementIdx) {
    float d = distance(cellPos, center);
    float radius = age * 25.0; // 波紋の広がり幅
    float ring = 1.0 - smoothstep(0.0, 3.0, abs(d - radius));
    float fade = 1.0 - age;
    float centerGlow = smoothstep(5.0, 0.0, d) * fade;
    return (ring * fade * 0.7) + (centerGlow * 0.3);
}

vec3 injectionColor(float elementIdx) {
    int idx = int(elementIdx);
    if (idx == 0) return vec3(0.2, 1.0, 0.4);  // C: エメラルド
    if (idx == 1) return vec3(0.2, 0.5, 1.0);  // N: コバルト
    if (idx == 2) return vec3(1.0, 0.5, 0.1);  // P: アンバー
    if (idx == 3) return vec3(0.8, 0.2, 1.0);  // H: パープル
    return vec3(0.0, 1.0, 1.0);                // fallback
}

void main() {
    // セル座標とローカル座標の算出
    ivec2 cellCoord = ivec2(floor(v_uv * u_grid_size));
    vec2 cellUV = fract(v_uv * u_grid_size);
    vec2 cellPos = v_uv * u_grid_size;

    // 範囲外クランプ
    cellCoord = clamp(cellCoord, ivec2(0), ivec2(u_grid_size) - 1);

    // ホバー判定
    bool isHovered = (cellCoord.x == int(u_hover_cell.x) && cellCoord.y == int(u_hover_cell.y));

    // リップルの合算
    float totalRipple = 0.0;
    vec3 rippleColor = vec3(0.0);
    for (int i = 0; i < 4; i++) {
        if (i >= u_injection_count) break;
        vec4 inj = u_injection_centers[i];
        float ripple = injectionRipple(cellPos, inj.xy, inj.z, inj.w);
        if (ripple > 0.001) {
            vec3 ic = injectionColor(inj.w);
            rippleColor += ic * ripple;
            totalRipple += ripple;
        }
    }
    totalRipple = clamp(totalRipple, 0.0, 1.0);

    // セル内グリッド枠線
    float border = step(0.93, cellUV.x) + step(0.93, cellUV.y) + step(cellUV.x, 0.07) + step(cellUV.y, 0.07);
    border = clamp(border, 0.0, 1.0);

    // 3×3近傍のメタボール融合場
    float field = 0.0;
    vec3 fieldColor = vec3(0.0);
    float fieldWeight = 0.0;
    float morphSum = 0.0;
    float morphWeight = 0.0;

    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            ivec2 nc = cellCoord + ivec2(dx, dy);
            if (nc.x >= 0 && nc.x < int(u_grid_size.x) && nc.y >= 0 && nc.y < int(u_grid_size.y)) {
                vec4 nd = texelFetch(u_gridTex0, nc, 0);
                if (nd.b > 0.5) { // Active cell
                    vec2 center = vec2(dx, dy) + vec2(0.5) - cellUV;
                    float d = length(center);
                    float contribution = 1.0 / (d * d + 0.15);
                    field += contribution;

                    vec3 ncColor = getElementColor(nc);
                    fieldColor += ncColor * contribution;
                    fieldWeight += contribution;

                    morphSum += nd.a * contribution;
                    morphWeight += contribution;
                }
            }
        }
    }

    // ブロブの決定閾値
    float blob = smoothstep(3.5, 4.5, field);

    if (blob < 0.01) {
        // 非アメーバ（背景）領域の描画
        float voronoi = cellularNoise(v_uv * 18.0);
        // シックで深みのあるサイバー背景
        vec3 bgBase = mix(vec3(0.012, 0.016, 0.026), vec3(0.02, 0.035, 0.055), (1.0 - voronoi) * 0.35);
        vec3 finalBg = mix(bgBase, bgBase * 1.6, border * 0.25);

        if (isHovered) {
            // ホバー時の非アクティブセル枠線発光
            finalBg += vec3(0.04, 0.18, 0.28) * (border * 0.8 + 0.2);
        }

        if (totalRipple > 0.001) {
            finalBg += rippleColor * 0.35;
        }

        fragColor = vec4(finalBg, 1.0);
        return;
    }

    // アメーバ融合色
    vec3 color = fieldColor / max(fieldWeight, 0.001);
    float morph = morphSum / max(morphWeight, 0.001);

    // --- 擬似法線の算出 (3D陰影用) ---
    float eps = 0.003;
    float fx = getFieldAtUV(v_uv + vec2(eps, 0.0)) - getFieldAtUV(v_uv - vec2(eps, 0.0));
    float fy = getFieldAtUV(v_uv + vec2(0.0, eps)) - getFieldAtUV(v_uv - vec2(0.0, eps));
    vec3 normal = normalize(vec3(-fx, -fy, 0.15));
    
    // --- Phong ライティング ---
    vec3 lightDir = normalize(vec3(0.3, 0.4, 1.0));
    float diffuse = max(dot(normal, lightDir), 0.0);
    vec3 viewDir = vec3(0.0, 0.0, 1.0);
    vec3 halfDir = normalize(lightDir + viewDir);
    float specular = pow(max(dot(normal, halfDir), 0.0), 48.0);
    
    // ライティングの適用（柔らかいベース拡散光 + ハイライト）
    color = color * (0.35 + 0.65 * diffuse) + vec3(1.0) * specular * 0.25;

    // 形態 (morphology) 別の内部有機パターン & 形態固有色味シフト
    float pattern = 0.0;
    if (morph < 0.5) {
        // Basic: 微粒子パターン
        pattern = step(0.88, fract(cellUV.x * 3.0)) * step(0.88, fract(cellUV.y * 3.0));
        pattern *= 0.20;
    } else if (morph < 1.5) {
        // Producer (光合成): 同心円の脈動 + 緑みの色味シフト
        float rings = sin(length(cellUV - 0.5) * 22.0 - u_time * 5.0) * 0.5 + 0.5;
        pattern = rings * 0.30;
        color = mix(color, color * vec3(0.8, 1.2, 0.85), 0.15 * blob);
    } else if (morph < 2.5) {
        // Consumer: 波動 + 暖色シフト
        float waves = sin(cellUV.x * 12.0 + cellUV.y * 12.0 + u_time * 6.0) * 0.5 + 0.5;
        pattern = waves * 0.25;
        color = mix(color, color * vec3(1.2, 1.0, 0.8), 0.15 * blob);
    } else if (morph < 3.5) {
        // Predator: 放射状トゲ + 赤みシフト
        float angle = atan(cellUV.y - 0.5, cellUV.x - 0.5);
        float cellDist = length(cellUV - 0.5);
        float radial = sin(angle * 8.0 + u_time * 7.0) * 0.5 + 0.5;
        radial *= (1.0 - cellDist * 1.5);
        pattern = clamp(radial, 0.0, 1.0) * 0.35;
        color = mix(color, color * vec3(1.3, 0.85, 0.85), 0.15 * blob);
    } else {
        // Decomposer: 渦巻き + 紫シフト
        float angle = atan(cellUV.y - 0.5, cellUV.x - 0.5);
        float r = length(cellUV - 0.5);
        float spiral = sin(angle * 4.0 - r * 18.0 + u_time * 4.0) * 0.5 + 0.5;
        pattern = spiral * 0.28;
        color = mix(color, color * vec3(1.1, 0.85, 1.2), 0.15 * blob);
    }

    // パターンのブレンド
    color = mix(color, color * 1.45, pattern * blob);

    // --- 細胞核と細胞膜の微細構造 ---
    float cellDist = length(cellUV - 0.5);
    // 核: 中心の輝点
    float nucleus = smoothstep(0.18, 0.10, cellDist) * 0.30;
    // 膜: 外周 of アメーバ（半透明リング）
    float membrane = smoothstep(0.48, 0.42, cellDist) * smoothstep(0.36, 0.42, cellDist) * 0.25;
    color += color * (nucleus + membrane);

    // --- フレネルリムライト ---
    float fresnel = pow(1.0 - max(dot(normal, viewDir), 0.0), 3.0);
    color += fresnel * vec3(0.12, 0.30, 0.50) * 0.5;
    
    // メタボール境界の柔らかなグロー
    float edge = smoothstep(4.5, 3.5, field) * smoothstep(1.5, 3.5, field);
    color += color * edge * 0.3;

    // エネルギーフローの可視化
    vec2 gradient = vec2(
        getEnergy(cellCoord + ivec2(1, 0)) - getEnergy(cellCoord - ivec2(1, 0)),
        getEnergy(cellCoord + ivec2(0, 1)) - getEnergy(cellCoord - ivec2(0, 1))
    );
    float gradLen = length(gradient);
    if (gradLen > 15.0) {
        vec2 gradDir = normalize(gradient);
        float flow = sin(dot(gradDir, cellUV - 0.5) * 5.0 - u_time * 9.0) * 0.5 + 0.5;
        color += vec3(0.04, 0.1, 0.16) * flow * blob * clamp(gradLen / 80000.0, 0.0, 1.0);
    }

    // 注入リップルフラッシュ効果
    if (totalRipple > 0.001) {
        color = mix(color, rippleColor, totalRipple * 0.45);
    }

    // ホバーハイライト
    if (isHovered) {
        color = mix(color, vec3(0.1, 1.0, 1.0), border * 0.8 + 0.25);
    }

    // 背景色とアメーバ色をブレンド
    float voronoi = cellularNoise(v_uv * 18.0);
    vec3 bgBase = mix(vec3(0.012, 0.016, 0.026), vec3(0.02, 0.035, 0.055), (1.0 - voronoi) * 0.35);
    vec3 finalColor = mix(bgBase, color, blob);

    fragColor = vec4(finalColor, 1.0);
}

