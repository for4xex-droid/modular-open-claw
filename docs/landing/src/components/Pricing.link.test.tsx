/*
 * Verification-only: Pro Payment Link wiring (2026-07-05)
 */
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { Pricing } from './Pricing';
import i18n from '../i18n/config';

const NEW_LINK = 'https://buy.stripe.com/aFa00i9cEaVE4ay4y9f7i03';
const OLD_LINK = 'https://buy.stripe.com/aFa9AS1Kc1l47mK3u5f7i01';

describe('Pricing Payment Link wiring', () => {
  it('Pro CTA uses the $19.99 Payment Link with safe external attrs', async () => {
    await i18n.changeLanguage('en');
    render(<Pricing />);
    const pro = screen.getByRole('link', { name: /Upgrade to Pro/i });
    expect(pro).toHaveAttribute('href', NEW_LINK);
    expect(pro).toHaveAttribute('target', '_blank');
    expect(pro).toHaveAttribute('rel', 'noopener noreferrer');
    expect(pro.getAttribute('href')).not.toBe(OLD_LINK);
  });
});
