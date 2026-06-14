import { render } from '@testing-library/react';
import FluidBackground from './FluidBackground';
import { useFluidConfig } from './useFluidConfig';
import { cssVar } from '../../utils/cssVar';

// Mock useFluidConfig
jest.mock('./useFluidConfig');
const mockUseFluidConfig = useFluidConfig as jest.Mock;

describe('FluidBackground', () => {
  beforeEach(() => {
    mockUseFluidConfig.mockReturnValue({
      enabled: true,
      simResolution: 128,
      dyeResolution: 512,
      intensity: 0.3,
      colors: {
        primary: cssVar('--fluid-warm-ivory', '#d4c5a9'),
        secondary: cssVar('--fluid-deep-gold', '#b8965a'),
        primaryRGB: [1, 1, 1],
        secondaryRGB: [0, 0, 0]
      },
      maxDpr: 1.5
    });
  });

  it('should render canvas when enabled', () => {
    const { container } = render(<FluidBackground />);
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('should not render canvas when disabled', () => {
    mockUseFluidConfig.mockReturnValue({
      enabled: false,
      colors: {
        primary: cssVar('--fluid-warm-ivory', '#d4c5a9'),
        secondary: cssVar('--fluid-deep-gold', '#b8965a'),
        primaryRGB: [1, 1, 1],
        secondaryRGB: [0, 0, 0]
      }
    });

    const { container } = render(<FluidBackground />);
    const canvas = container.querySelector('canvas');
    expect(canvas).not.toBeInTheDocument();
  });
});
