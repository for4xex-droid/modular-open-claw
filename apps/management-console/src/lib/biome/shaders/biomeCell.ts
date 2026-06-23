// --- 頂点シェーダー ---
export const biomeCellVertexShader = /* glsl */`
  varying vec2 vUv;
  varying vec3 vColor;
  varying vec3 vNormal;
  varying vec3 vWorldPos;

  void main() {
    vUv = uv;  // ジオメトリの UV -> セル内ローカル座標 (0-1)
    vColor = instanceColor;
    vNormal = normalMatrix * normal;

    vec4 worldPos = instanceMatrix * vec4(position, 1.0);
    vWorldPos = worldPos.xyz;
    gl_Position = projectionMatrix * viewMatrix * worldPos;
  }
`;

// --- フラグメントシェーダー ---
export const biomeCellFragmentShader = /* glsl */`
  precision highp float;

  uniform float u_time;
  uniform int u_morphType;
  uniform int u_rarity;

  varying vec2 vUv;
  varying vec3 vColor;
  varying vec3 vNormal;
  varying vec3 vWorldPos;

  void main() {
    vec3 color = vColor;  // instanceColor

    // --- Phong ライティング ---
    vec3 normal = normalize(vNormal);
    vec3 lightDir = normalize(vec3(0.3, 0.4, 1.0));
    float diffuse = max(dot(normal, lightDir), 0.0);
    vec3 viewDir = vec3(0.0, 0.0, 1.0);
    vec3 halfDir = normalize(lightDir + viewDir);
    float specular = pow(max(dot(normal, halfDir), 0.0), 48.0);
    color = color * (0.35 + 0.65 * diffuse) + vec3(1.0) * specular * 0.25;

    // --- 形態別内部パターン ---
    float pattern = 0.0;
    vec2 cellUV = vUv;

    if (u_morphType == 0) {
      // Basic: 微粒子パターン
      pattern = step(0.88, fract(cellUV.x * 3.0)) * step(0.88, fract(cellUV.y * 3.0));
      pattern *= 0.20;
    } else if (u_morphType == 1) {
      // Producer: 同心円の脈動 + 緑みシフト
      float rings = sin(length(cellUV - 0.5) * 22.0 - u_time * 5.0) * 0.5 + 0.5;
      pattern = rings * 0.30;
      color = mix(color, color * vec3(0.8, 1.2, 0.85), 0.15);
    } else if (u_morphType == 2) {
      // Consumer: 波動 + 暖色シフト
      float waves = sin(cellUV.x * 12.0 + cellUV.y * 12.0 + u_time * 6.0) * 0.5 + 0.5;
      pattern = waves * 0.25;
      color = mix(color, color * vec3(1.2, 1.0, 0.8), 0.15);
    } else if (u_morphType == 3) {
      // Predator: 放射状トゲ + 赤みシフト
      float angle = atan(cellUV.y - 0.5, cellUV.x - 0.5);
      float cellDist = length(cellUV - 0.5);
      float radial = sin(angle * 8.0 + u_time * 7.0) * 0.5 + 0.5;
      radial *= (1.0 - cellDist * 1.5);
      pattern = clamp(radial, 0.0, 1.0) * 0.35;
      color = mix(color, color * vec3(1.3, 0.85, 0.85), 0.15);
    } else {
      // Decomposer: 渦巻き + 紫シフト
      float angle = atan(cellUV.y - 0.5, cellUV.x - 0.5);
      float r = length(cellUV - 0.5);
      float spiral = sin(angle * 4.0 - r * 18.0 + u_time * 4.0) * 0.5 + 0.5;
      pattern = spiral * 0.28;
      color = mix(color, color * vec3(1.1, 0.85, 1.2), 0.15);
    }

    // パターン適用
    color = mix(color, color * 1.45, pattern);

    // --- 細胞核と細胞膜 ---
    float cellDist = length(cellUV - 0.5);
    float nucleus = smoothstep(0.18, 0.10, cellDist) * 0.30;
    float membrane = smoothstep(0.48, 0.42, cellDist) * smoothstep(0.36, 0.42, cellDist) * 0.25;
    color += color * (nucleus + membrane);

    // --- フレネルリムライト ---
    float fresnel = pow(1.0 - max(dot(normal, viewDir), 0.0), 3.0);
    color += fresnel * vec3(0.12, 0.30, 0.50) * 0.5;

    // --- レアリティ別エフェクト ---
    if (u_rarity >= 3) {
      // Epic+: clearcoat 風の光沢
      float rimGlow = pow(1.0 - max(dot(normal, viewDir), 0.0), 5.0);
      color += rimGlow * vec3(0.2, 0.4, 0.6) * 0.3;
    }
    if (u_rarity >= 4) {
      // Legendary: 金色の脈動
      float legendaryPulse = sin(u_time * 3.0 + cellDist * 12.0) * 0.5 + 0.5;
      color += vec3(0.3, 0.2, 0.05) * legendaryPulse * 0.15;
    }

    gl_FragColor = vec4(color, 1.0);
  }
`;
