import { HiggsEffect } from './HiggsEffect';
import { Effect } from 'postprocessing';
import { Vector2 } from 'three';

describe('HiggsEffect', () => {
  it('should construct correctly with default parameters', () => {
    const effect = new HiggsEffect();
    expect(effect).toBeInstanceOf(Effect);
    expect(effect.uniforms.has('intensity')).toBe(true);
    expect(effect.uniforms.has('center')).toBe(true);
    expect(effect.uniforms.get('intensity')?.value).toBe(0.5);
    const center = effect.uniforms.get('center')?.value as Vector2;
    expect(center.x).toBe(0.5);
    expect(center.y).toBe(0.5);
  });

  it('should allow setting intensity and center values', () => {
    const effect = new HiggsEffect({ intensity: 0.8, center: [0.3, 0.4] });
    expect(effect.uniforms.get('intensity')?.value).toBe(0.8);
    let center = effect.uniforms.get('center')?.value as Vector2;
    expect(center.x).toBe(0.3);
    expect(center.y).toBe(0.4);

    effect.intensity = 0.2;
    effect.center = [0.7, 0.9];
    expect(effect.uniforms.get('intensity')?.value).toBe(0.2);
    center = effect.uniforms.get('center')?.value as Vector2;
    expect(center.x).toBe(0.7);
    expect(center.y).toBe(0.9);
  });
});
