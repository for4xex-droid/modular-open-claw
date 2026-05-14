import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import TaskApprovalOverlay from './TaskApprovalOverlay';
import { authenticatedFetch } from '../lib/auth';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { useToast } from './common/Toast';

// Mock dependencies
jest.mock('../lib/auth', () => ({
  authenticatedFetch: jest.fn()
}));

jest.mock('../i18n', () => ({
  useTranslation: () => ({ t: (key: string, vars?: Record<string, unknown>) => key })
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000'
}));

jest.mock('../hooks/useSystemVitality', () => ({
  useSystemVitality: jest.fn()
}));

jest.mock('./common/Toast', () => ({
  useToast: jest.fn()
}));

describe('TaskApprovalOverlay', () => {
  const mockShowToast = jest.fn();
  let consoleErrorSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    (useSystemVitality as jest.Mock).mockReturnValue({ lastEvent: null });
    (useToast as jest.Mock).mockReturnValue({ showToast: mockShowToast });
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('正常系: 初期ロードでAwaitingジョブがある場合にオーバーレイを表示', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation(() => Promise.resolve({
      ok: true,
      json: async () => [{ id: 'job-123', error_message: 'Needs manual approval' }]
    }));

    render(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.getByText('approval.title')).toBeInTheDocument();
    });

    expect(screen.getByText('job-123')).toBeInTheDocument();
    expect(screen.getByText('Needs manual approval')).toBeInTheDocument();
  });

  it('エッジケース: Awaitingジョブがない場合は何も表示しない', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation(() => Promise.resolve({
      ok: true,
      json: async () => []
    }));

    render(<TaskApprovalOverlay />);

    // 非同期処理を待つ
    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalled();
    });

    expect(screen.queryByText('approval.title')).not.toBeInTheDocument();
  });

  it('アクション: SSEイベントを受信した際にオーバーレイが表示される', async () => {
    // 初回ロードは空
    (authenticatedFetch as jest.Mock).mockImplementation(() => Promise.resolve({
      ok: true,
      json: async () => []
    }));

    const { rerender } = render(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.queryByText('approval.title')).not.toBeInTheDocument();
    });

    // SSEイベント発火
    (useSystemVitality as jest.Mock).mockReturnValue({
      lastEvent: {
        type: 'task_awaiting_input',
        data: { job_id: 'job-999', reason: 'Dangerous action detected' }
      }
    });

    rerender(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.getByText('job-999')).toBeInTheDocument();
      expect(screen.getByText('Dangerous action detected')).toBeInTheDocument();
    });
  });

  it('アクション: Approveボタン押下でジョブが承認されオーバーレイが消える', async () => {
    let reviewDone = false;
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/review')) {
        reviewDone = true;
        return Promise.resolve({ ok: true });
      }
      return Promise.resolve({
        ok: true,
        json: async () => reviewDone ? [] : [{ id: 'job-123', error_message: 'Needs manual approval' }]
      });
    });

    render(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.getByText('approval.title')).toBeInTheDocument();
    });

    // コメント入力
    const textarea = screen.getByPlaceholderText('approval.comments');
    fireEvent.change(textarea, { target: { value: 'Looks good' } });

    // Review送信モック (上記 mockImplementation で対応済み)

    const approveBtn = screen.getByText('approval.approve');
    fireEvent.click(approveBtn);

    await waitFor(() => {
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/v1/jobs/job-123/review',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ status: 'approved', comments: 'Looks good' })
        })
      );
    });

    // オーバーレイが消えることを確認
    await waitFor(() => {
      expect(screen.queryByText('approval.title')).not.toBeInTheDocument();
    });
  });
  
  it('異常系: エラーハンドリング (Review送信失敗時)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/review')) {
        return Promise.resolve({ 
          ok: false,
          status: 500,
          text: async () => 'Internal Server Error'
        });
      }
      return Promise.resolve({
        ok: true,
        json: async () => [{ id: 'job-123', error_message: 'Needs manual approval' }]
      });
    });

    render(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.getByText('approval.title')).toBeInTheDocument();
    });

    // Review送信モック失敗 (上記 mockImplementation で対応済み)

    const rejectBtn = screen.getByText('approval.reject');
    fireEvent.click(rejectBtn);

    await waitFor(() => {
      // C-3: reject時のPOSTリクエストの body を検証（コメント未入力 → comments省略）
      expect(authenticatedFetch).toHaveBeenCalledWith(
        'http://localhost:3000/api/v1/jobs/job-123/review',
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ status: 'rejected' })
        })
      );
    });

    await waitFor(() => {
      // C-2: t() は key をそのまま返すモックなので、interpolation args は t() 内で消費される
      // 実コンポーネントは showToast('error', t('approval.error_submit', { status: 500 }))
      // t() モック → 'approval.error_submit' を返す
      expect(mockShowToast).toHaveBeenCalledWith('error', 'approval.error_submit');
    });

    // エラー時はまだ表示されている
    expect(screen.getByText('approval.title')).toBeInTheDocument();
  });

  it('異常系: ネットワーク例外 (review送信時にfetchがthrowした場合)', async () => {
    (authenticatedFetch as jest.Mock).mockImplementation((url: string) => {
      if (url.includes('/review')) {
        return Promise.reject(new Error('Network error'));
      }
      return Promise.resolve({
        ok: true,
        json: async () => [{ id: 'job-123', error_message: 'Needs manual approval' }]
      });
    });

    render(<TaskApprovalOverlay />);

    await waitFor(() => {
      expect(screen.getByText('approval.title')).toBeInTheDocument();
    });

    const approveBtn = screen.getByText('approval.approve');
    fireEvent.click(approveBtn);

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith('error', 'approval.error_network');
    });

    // ネットワーク例外時もオーバーレイは表示されたまま
    expect(screen.getByText('approval.title')).toBeInTheDocument();
  });
});
