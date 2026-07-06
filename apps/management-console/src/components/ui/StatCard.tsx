/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';

export interface StatCardProps {
  label: string;
  value: React.ReactNode;
  trend?: React.ReactNode;
  trendClassName?: string;
  className?: string;
}

/** U6-8(1): App.css `.stat-card` パターンの再利用可能ラッパー */
export const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  trend,
  trendClassName = '',
  className = '',
}) => (
  <div className={`stat-card ${className}`.trim()}>
    <div className="stat-label">{label}</div>
    <div className="stat-value">{value}</div>
    {trend != null && (
      <div className={`stat-trend ${trendClassName}`.trim()}>{trend}</div>
    )}
  </div>
);
