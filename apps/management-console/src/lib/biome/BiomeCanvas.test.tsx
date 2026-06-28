/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import { BiomeCanvas } from './BiomeCanvas';

// R3F 関連をモック
jest.mock('@react-three/fiber', () => ({
  Canvas: ({ children, ...props }: any) => <div data-testid="r3f-canvas" {...props}>{children}</div>,
  useFrame: jest.fn(),
  extend: jest.fn(),
}));

jest.mock('@react-three/drei', () => ({
  Sparkles: (props: any) => <div data-testid="sparkles" {...props} />,
  shaderMaterial: jest.fn(() => function MockMaterial() {}),
  OrthographicCamera: (props: any) => <div data-testid="orthographic-camera" {...props} />,
}));

jest.mock('@react-three/postprocessing', () => ({
  EffectComposer: ({ children }: any) => <div data-testid="effect-composer">{children}</div>,
  Bloom: (props: any) => <div data-testid="bloom" {...props} />,
}));

jest.mock('postprocessing', () => ({
  Effect: class MockEffect {
    uniforms: Map<string, { value: any }>;
    constructor(name: string, shader: string, options: any = {}) {
      this.uniforms = options.uniforms || new Map();
    }
    dispose() {}
  },
  SavePass: jest.fn().mockImplementation(() => ({
    render: jest.fn(),
    dispose: jest.fn(),
  })),
  CopyPass: jest.fn().mockImplementation(() => ({
    render: jest.fn(),
    dispose: jest.fn(),
  })),
}));

describe('BiomeCanvas', () => {
  const dummyRenderView = new Float32Array(128 * 128 * 12);

  it('should render Canvas component', () => {
    render(
      <BiomeCanvas
        width={512}
        height={512}
        renderView={dummyRenderView}
        rarity={0}
      />
    );
    expect(screen.getByTestId('r3f-canvas')).toBeInTheDocument();
  });

  it('should render Sparkles only for Legendary rarity (4)', () => {
    const { rerender } = render(
      <BiomeCanvas
        width={512}
        height={512}
        renderView={dummyRenderView}
        rarity={3}
      />
    );
    expect(screen.queryByTestId('sparkles')).not.toBeInTheDocument();

    rerender(
      <BiomeCanvas
        width={512}
        height={512}
        renderView={dummyRenderView}
        rarity={4}
      />
    );
    expect(screen.getByTestId('sparkles')).toBeInTheDocument();
  });
});
