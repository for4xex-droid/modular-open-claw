/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import EscrowManagementView from './EscrowManagementView';
import { authenticatedFetch } from '../lib/auth';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000'
}));

const AGENT_ID = '11111111-1111-1111-1111-111111111111';

const mockEscrows = [
  {
    id: 'escrow-1234',
    status: 'Locked',
    amount: 1500,
    created_at: '2026-05-15T00:00:00Z',
    payee_id: 'agent-b'
  },
  {
    id: 'escrow-5678',
    status: 'Released',
    amount: 300,
    created_at: '2026-05-14T00:00:00Z',
    payee_id: 'agent-b'
  }
];

describe('EscrowManagementView', () => {
  let consoleErrorSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('正常系: 期待される動作 (Loadingからデータ表示まで)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/api/v1/commerce/escrow/history/')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve(mockEscrows) });
      }
      return Promise.resolve({ ok: false });
    });

    render(<EscrowManagementView agentId={AGENT_ID} />);

    // 最初はローディング表示
    expect(screen.getByText('common.loading')).toBeInTheDocument();

    // データがフェッチされて表示されるのを待つ
    await waitFor(() => {
      expect(screen.getByText('escrow-1...')).toBeInTheDocument();
    });

    expect(screen.getByText('1500')).toBeInTheDocument();
    expect(screen.getByText('Locked')).toBeInTheDocument();
    expect(screen.getByText('escrow-5...')).toBeInTheDocument();
    expect(screen.getByText('Released')).toBeInTheDocument();
  });

  it('異常系: エラーハンドリング (API失敗時)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation(() => 
      Promise.resolve({ ok: false })
    );

    render(<EscrowManagementView agentId={AGENT_ID} />);

    await waitFor(() => {
      expect(screen.getByText('escrow.loadFailed')).toBeInTheDocument();
    });
  });

  it('エッジケース: 空入力 (データ0件)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/api/v1/commerce/escrow/history/')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve([]) });
      }
      return Promise.resolve({ ok: false });
    });

    render(<EscrowManagementView agentId={AGENT_ID} />);

    await waitFor(() => {
      expect(screen.getByText('escrow.noData')).toBeInTheDocument();
    });
  });

  it('アクション: Releaseボタン押下でPOSTリクエストが飛ぶか', async () => {
    let callCount = 0;
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/release')) {
        return Promise.resolve({ ok: true });
      }
      if (url.includes('/history/')) {
        callCount++;
        // 2回目の取得（Release後）はReleasedのデータを返す
        const escrowsToReturn = callCount > 1 
          ? [{ ...mockEscrows[0], status: 'Released' }] 
          : [mockEscrows[0]];
        return Promise.resolve({ ok: true, json: () => Promise.resolve(escrowsToReturn) });
      }
      return Promise.resolve({ ok: false });
    });

    render(<EscrowManagementView agentId={AGENT_ID} />);

    // ボタンが表示されるのを待つ
    await waitFor(() => {
      expect(screen.getByText('escrow.release')).toBeInTheDocument();
    });

    const releaseButton = screen.getByText('escrow.release');
    fireEvent.click(releaseButton);

    await waitFor(() => {
      // POSTリクエストが正しいURLとボディで呼ばれたか確認
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/v1/commerce/escrow/escrow-1234/release',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ payee_id: AGENT_ID })
        })
      );
    });

    // 再取得後のステータス更新を待つ
    await waitFor(() => {
      expect(screen.getByText('Released')).toBeInTheDocument();
    });
  });

  it('異常系: ネットワーク例外 (fetch が throw した場合)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation(() =>
      Promise.reject(new Error('Network error'))
    );

    render(<EscrowManagementView agentId={AGENT_ID} />);

    await waitFor(() => {
      expect(screen.getByText('escrow.loadFailed')).toBeInTheDocument();
    });
  });
});
