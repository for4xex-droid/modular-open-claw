/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen } from '@testing-library/react';
import App from './App';
import './i18n/config';

describe('App Routing', () => {
  const originalLocation = window.location;

  beforeAll(() => {
    // window.location の一部をモックできるようにする
    Object.defineProperty(window, 'location', {
      configurable: true,
      writable: true,
      value: { ...originalLocation, pathname: '/' },
    });
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      configurable: true,
      writable: true,
      value: originalLocation,
    });
  });

  it('renders landing page content on root path', () => {
    window.location.pathname = '/';
    render(<App />);
    // メインのHeroコンテンツの一部が表示されることを確認
    const hasHeroText = screen.queryByText(/Self-Healing AI Agent with Soul System/i) || 
                        screen.queryByText(/自己修復型AIエージェント/i);
    expect(hasHeroText).toBeInTheDocument();
  });

  it('renders privacy policy on /privacy', () => {
    window.location.pathname = '/privacy';
    render(<App />);
    const hasPrivacyTitle = screen.queryByText(/Privacy Policy/i) || 
                            screen.queryByText(/プライバシーポリシー/i);
    expect(hasPrivacyTitle).toBeInTheDocument();
    // メインのランディングコンテンツが表示されていないことを確認
    const hasHeroText = screen.queryByText(/Self-Healing AI Agent with Soul System/i) || 
                        screen.queryByText(/自己修復型AIエージェント/i);
    expect(hasHeroText).not.toBeInTheDocument();
  });

  it('renders terms of service on /terms', () => {
    window.location.pathname = '/terms';
    render(<App />);
    const hasTermsTitle = screen.queryByText(/Terms of Service/i) || 
                          screen.queryByText(/利用規約/i);
    expect(hasTermsTitle).toBeInTheDocument();
    const hasHeroText = screen.queryByText(/Self-Healing AI Agent with Soul System/i) || 
                        screen.queryByText(/自己修復型AIエージェント/i);
    expect(hasHeroText).not.toBeInTheDocument();
  });

  it('renders tokushoho on /tokushoho', () => {
    window.location.pathname = '/tokushoho';
    render(<App />);
    const hasTokushohoTitle = screen.queryAllByText(/特定商取引法/i).length > 0 || 
                              screen.queryAllByText(/Specified Commercial/i).length > 0;
    expect(hasTokushohoTitle).toBe(true);
    const hasHeroText = screen.queryByText(/Self-Healing AI Agent with Soul System/i) || 
                        screen.queryByText(/自己修復型AIエージェント/i);
    expect(hasHeroText).not.toBeInTheDocument();
  });
});
