/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Effect } from 'postprocessing';
import { Uniform, Vector2 } from 'three';

const fragmentShader = `
  uniform float intensity;
  uniform vec2 center;

  void mainImage(const in vec4 inputColor, const in vec2 uv, out vec4 outputColor) {
    // higgs.frag: 重力レンズ + RGB チャンネル分離
    vec2 dir = uv - center;
    float dist = length(dir);
    // 重力歪みの強さ
    float distortion = intensity * 0.05 / (dist + 0.05);
    vec2 offset = normalize(dir) * distortion;
    
    // RGB チャンネル分離 (色収差)
    float r = texture2D(inputBuffer, uv - offset).r;
    float g = texture2D(inputBuffer, uv).g;
    float b = texture2D(inputBuffer, uv + offset).b;
    
    outputColor = vec4(r, g, b, inputColor.a);
  }
`;

export class HiggsEffect extends Effect {
  constructor({ intensity = 0.5, center = [0.5, 0.5] }: { intensity?: number; center?: [number, number] } = {}) {
    super('HiggsEffect', fragmentShader, {
      uniforms: new Map<string, Uniform<any>>([
        ['intensity', new Uniform(intensity)],
        ['center', new Uniform(new Vector2(center[0], center[1]))],
      ]),
    });
  }

  get intensity(): number {
    return this.uniforms.get('intensity')!.value;
  }

  set intensity(value: number) {
    this.uniforms.get('intensity')!.value = value;
  }

  get center(): [number, number] {
    const uCenter = this.uniforms.get('center')!.value as Vector2;
    return [uCenter.x, uCenter.y];
  }

  set center(value: [number, number]) {
    const uCenter = this.uniforms.get('center')!.value as Vector2;
    uCenter.set(value[0], value[1]);
  }
}
