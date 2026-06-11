/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { CycleSelect } from './CycleSelect';

describe('CycleSelect Component', () => {
  it('正しい状態（paused と speed）を描画すること', () => {
    const handleSpeedChange = jest.fn();
    const handleTogglePause = jest.fn();

    const { rerender } = render(
      <CycleSelect
        speed={100}
        onSpeedChange={handleSpeedChange}
        paused={false}
        onTogglePause={handleTogglePause}
      />
    );

    // 一時停止状態ではないので、ボタンは "Pause" であるべき
    const pauseBtn = screen.getByRole('button', { name: /Pause/i });
    expect(pauseBtn).toBeInTheDocument();

    // 1x ボタンがアクティブであること (インジケータや何かしらのスタイルがあるか等、描画されていることを確認)
    const btn1x = screen.getByRole('button', { name: '1x' });
    expect(btn1x).toBeInTheDocument();

    // paused=true で再描画
    rerender(
      <CycleSelect
        speed={100}
        onSpeedChange={handleSpeedChange}
        paused={true}
        onTogglePause={handleTogglePause}
      />
    );

    const resumeBtn = screen.getByRole('button', { name: /Resume/i });
    expect(resumeBtn).toBeInTheDocument();
  });

  it('Resume/Pause ボタンのクリックで onTogglePause が呼ばれること', () => {
    const handleTogglePause = jest.fn();
    render(
      <CycleSelect
        speed={100}
        onSpeedChange={jest.fn()}
        paused={false}
        onTogglePause={handleTogglePause}
      />
    );

    const pauseBtn = screen.getByRole('button', { name: /Pause/i });
    fireEvent.click(pauseBtn);
    expect(handleTogglePause).toHaveBeenCalledTimes(1);
  });

  it('速度ボタン（1x, 2x, 5x, 10x）をクリックすると onSpeedChange が呼ばれること', () => {
    const handleSpeedChange = jest.fn();
    render(
      <CycleSelect
        speed={100}
        onSpeedChange={handleSpeedChange}
        paused={false}
        onTogglePause={jest.fn()}
      />
    );

    const btn2x = screen.getByRole('button', { name: '2x' });
    fireEvent.click(btn2x);
    expect(handleSpeedChange).toHaveBeenCalledWith(50); // 2x は 50ms

    const btn5x = screen.getByRole('button', { name: '5x' });
    fireEvent.click(btn5x);
    expect(handleSpeedChange).toHaveBeenCalledWith(20); // 5x は 20ms
  });
});
