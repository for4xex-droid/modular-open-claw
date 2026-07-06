/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';

export interface CardProps {
  children: React.ReactNode;
  className?: string;
  variant?: 'glass' | 'stat';
  padding?: 'md' | 'lg';
  onClick?: () => void;
  'data-testid'?: string;
}

/** U6-8(1): glass-panel / stat-card の共通ラッパー */
export const Card: React.FC<CardProps> = ({
  children,
  className = '',
  variant = 'glass',
  padding = 'lg',
  onClick,
  'data-testid': testId,
}) => {
  const base = variant === 'stat' ? 'stat-card' : 'glass-panel ui-card';
  const pad = padding === 'md' ? 'ui-card--pad-md' : 'ui-card--pad-lg';
  const interactive = onClick ? ' ui-card--interactive' : '';
  const Tag = onClick ? 'button' : 'div';

  return (
    <Tag
      type={onClick ? 'button' : undefined}
      className={`${base} ${pad}${interactive} ${className}`.trim()}
      onClick={onClick}
      data-testid={testId}
    >
      {children}
    </Tag>
  );
};
