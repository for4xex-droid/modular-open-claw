
import { render, screen, fireEvent } from '@testing-library/react';
import AvatarViewerModal from './AvatarViewerModal';

// Mock Framer Motion and Tree-related components to avoid WebGL errors in test
jest.mock('framer-motion', () => ({
  motion: {
    div: ({ children, style, ...props }: any) => <div style={style} {...props}>{children}</div>
  },
  AnimatePresence: ({ children }: any) => <>{children}</>
}));

jest.mock('@react-three/fiber', () => ({
  Canvas: ({ children }: any) => <div data-testid="mock-canvas">{children}</div>
}));

jest.mock('@react-three/drei', () => ({
  OrbitControls: () => null,
  Float: ({ children }: any) => <>{children}</>,
  MeshReflectorMaterial: () => null,
  Sparkles: () => null
}));

describe('AvatarViewerModal', () => {
  const mockOnClose = jest.fn();

  it('does not render when closed', () => {
    render(
      <AvatarViewerModal 
        isOpen={false} 
        onClose={mockOnClose} 
        modelUrl="test.vrm" 
        avatarState="idle" 
        mode="vrm" 
      />
    );
    expect(screen.queryByTestId('mock-canvas')).not.toBeInTheDocument();
  });

  it('renders modal content when open', () => {
    render(
      <AvatarViewerModal 
        isOpen={true} 
        onClose={mockOnClose} 
        modelUrl="test.vrm" 
        avatarState="idle" 
        mode="vrm" 
      />
    );
    expect(screen.getByTestId('mock-canvas')).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', () => {
    render(
      <AvatarViewerModal 
        isOpen={true} 
        onClose={mockOnClose} 
        modelUrl="test.vrm" 
        avatarState="idle" 
        mode="vrm" 
      />
    );
    const closeBtn = screen.getByRole('button');
    fireEvent.click(closeBtn);
    expect(mockOnClose).toHaveBeenCalled();
  });
});
