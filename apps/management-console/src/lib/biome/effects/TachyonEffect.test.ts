import { TachyonEffect } from './TachyonEffect';
import { Effect } from 'postprocessing';

describe('TachyonEffect', () => {
  it('should construct correctly with default parameters', () => {
    const effect = new TachyonEffect();
    expect(effect).toBeInstanceOf(Effect);
    expect(effect.uniforms.has('damp')).toBe(true);
    expect(effect.uniforms.get('damp')?.value).toBe(0.85);
  });

  it('should allow setting damp value', () => {
    const effect = new TachyonEffect({ damp: 0.5 });
    expect(effect.uniforms.get('damp')?.value).toBe(0.5);
    effect.damp = 0.99;
    expect(effect.uniforms.get('damp')?.value).toBe(0.99);
  });
});
