import * as THREE from 'three';

export function createCellGeometry(morph: number, rarity: number): THREE.BufferGeometry {
  // 3D ジオメトリの場合 (Epic以上)
  if (rarity === 3) {
    // Epic: 多面体。形態に応じて異なる多面体を使用
    switch (morph) {
      case 0: return new THREE.TetrahedronGeometry(0.35);
      case 1: return new THREE.OctahedronGeometry(0.35);
      case 2: return new THREE.DodecahedronGeometry(0.35);
      case 3: return new THREE.ConeGeometry(0.25, 0.5, 5);
      default: return new THREE.TorusGeometry(0.25, 0.08, 8, 24);
    }
  }
  
  if (rarity === 4) {
    // Legendary: 正二十面体または精密な多面体
    switch (morph) {
      case 0: return new THREE.IcosahedronGeometry(0.38);
      case 1: return new THREE.IcosahedronGeometry(0.38, 1); // 1段階細分化
      case 2: return new THREE.TorusKnotGeometry(0.22, 0.07, 32, 4);
      case 3: return new THREE.OctahedronGeometry(0.38);
      default: return new THREE.DodecahedronGeometry(0.38);
    }
  }

  // 2D 形状の場合 (Common, Uncommon, Rare)
  // Rare (2): 複雑な2D形状 (星型、歯車など)
  if (rarity === 2) {
    const shape = new THREE.Shape();
    const points = 5 + morph; // 形態ごとに角数を変える
    const outerRadius = 0.35;
    const innerRadius = 0.18;
    
    // 星型 / 歯車型の Shape を生成
    for (let i = 0; i < points * 2; i++) {
      const angle = (i * Math.PI) / points;
      const r = i % 2 === 0 ? outerRadius : innerRadius;
      const x = Math.cos(angle) * r;
      const y = Math.sin(angle) * r;
      if (i === 0) {
        shape.moveTo(x, y);
      } else {
        shape.lineTo(x, y);
      }
    }
    shape.closePath();
    return new THREE.ShapeGeometry(shape);
  }

  // Uncommon (1): リング状
  if (rarity === 1) {
    // 内径と外径を形態に応じて微調整
    const inner = 0.1 + morph * 0.03;
    const outer = 0.3 + morph * 0.02;
    return new THREE.RingGeometry(inner, outer, 16);
  }

  // Common (0): 単純な円
  // 形態ごとにセグメント数(角数)を変えて、五角形〜円を表現
  const segments = 5 + morph * 2;
  return new THREE.CircleGeometry(0.32, segments);
}
