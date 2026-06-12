import { render } from '@testing-library/react';
import FluidAura from './FluidAura';

// Mock react-three-fiber hooks since we are not inside a Canvas in tests
jest.mock('@react-three/fiber', () => ({
  useFrame: jest.fn()
}));

jest.mock('../../utils/cssVar', () => ({
  cssVar: jest.fn((name: string, fallback: string) => fallback)
}));

describe('FluidAura', () => {
  it('renders mesh', () => {
    // Tests R3F elements rendering in standard DOM context
    const { container } = render(
      <mesh>
        <FluidAura avatarState="idle" />
      </mesh>
    );
    // mesh/planeGeometry/shaderMaterial will render as custom elements in DOM under jest
    expect(container.firstChild).toBeInTheDocument();
  });
});
