/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';

export interface SectionHeaderProps {
  icon?: React.ReactNode;
  title: React.ReactNode;
  description?: React.ReactNode;
  className?: string;
}

/** U6-8(1): 設定/セットアップ画面のセクションヘッダ共通部品 */
export const SectionHeader: React.FC<SectionHeaderProps> = ({
  icon,
  title,
  description,
  className = '',
}) => (
  <div className={`ui-section-header ${className}`.trim()}>
    {icon && <span className="ui-section-header__icon">{icon}</span>}
    <div className="ui-section-header__text">
      <h3 className="ui-section-header__title">{title}</h3>
      {description && <p className="ui-section-header__desc">{description}</p>}
    </div>
  </div>
);
