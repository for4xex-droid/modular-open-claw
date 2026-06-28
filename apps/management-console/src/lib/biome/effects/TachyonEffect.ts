/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Effect, CopyPass } from 'postprocessing';
import { Uniform, WebGLRenderTarget } from 'three';

const fragmentShader = `
  uniform float damp;
  uniform sampler2D tHistory;

  void mainImage(const in vec4 inputColor, const in vec2 uv, out vec4 outputColor) {
    vec4 history = texture2D(tHistory, uv);
    // tachyon.frag L17: シアン色調の残像
    vec3 ghostColor = history.rgb * vec3(0.6, 0.95, 1.0);
    outputColor = vec4(mix(inputColor.rgb, ghostColor, damp), inputColor.a);
  }
`;

export class TachyonEffect extends Effect {
  private copyPass: CopyPass;
  private historyBuffer: WebGLRenderTarget;

  constructor({ damp = 0.85 }: { damp?: number } = {}) {
    const historyBuffer = new WebGLRenderTarget(1, 1);

    super('TachyonEffect', fragmentShader, {
      uniforms: new Map<string, Uniform<any>>([
        ['damp', new Uniform(damp)],
        ['tHistory', new Uniform(historyBuffer.texture)],
      ]),
    });

    this.historyBuffer = historyBuffer;
    this.copyPass = new CopyPass(historyBuffer, false);
  }

  update(
    renderer: any,
    inputBuffer: WebGLRenderTarget,
    _deltaTime?: number
  ): void {
    // サイズ同期
    const { width, height } = inputBuffer;
    if (this.historyBuffer.width !== width || this.historyBuffer.height !== height) {
      this.historyBuffer.setSize(width, height);
    }

    // CopyPass を使って現フレームを historyBuffer にコピー
    this.copyPass.render(renderer, inputBuffer, null);
  }

  get damp(): number {
    return this.uniforms.get('damp')!.value;
  }

  set damp(value: number) {
    this.uniforms.get('damp')!.value = value;
  }

  dispose(): void {
    super.dispose();
    this.historyBuffer.dispose();
    this.copyPass.dispose();
  }
}

