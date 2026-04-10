
import { render, screen } from '@testing-library/react';
import AiomeAvatar from './AiomeAvatar';

// Mock the hook
jest.mock('../hooks/AvatarContext', () => ({
  useAvatarCharacter: () => ({
    getAssetPath: (mode: string) => `test-path-${mode}.png`
  })
}));

describe('AiomeAvatar', () => {
  it('renders avatar image with correct src', () => {
    render(<AiomeAvatar status="idle" />);
    const img = screen.getByAltText('avatar.status.idle');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'test-path-lite.png');
  });

  it('applies sizes to container', () => {
    const { container } = render(<AiomeAvatar status="idle" size={200} />);
    const div = container.firstChild as HTMLElement;
    expect(div.style.width).toBe('200px');
    expect(div.style.height).toBe('200px');
  });
});
