/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { McpConfigManager } from './McpConfigManager';
import { authenticatedFetch } from '../lib/auth';

jest.mock('../lib/auth', () => ({
    authenticatedFetch: jest.fn(),
}));

jest.mock('../config', () => ({
    API_BASE: 'http://localhost'
}));

jest.mock('../i18n', () => ({
    useTranslation: () => ({
        t: (key: string, options?: any) => {
            const defaults: Record<string, string> = {
                'settings.mcpArchitecture': 'MCP Architecture (Analytics & Tools)',
                'settings.mcpDesc': 'Define external MCP servers',
                'settings.saveSyncTools': 'Save & Sync Tools',
                'settings.reloadedSuccessfully': 'Reloaded successfully',
                'settings.errorSaving': 'Error saving',
                'settings.invalidJson': 'Invalid JSON or network error'
            };
            return options?.defaultValue || defaults[key] || key;
        }
    })
}));

describe('McpConfigManager', () => {
    const mockConfig = {
        mcp_servers: {
            "test-server": {
                command: "node",
                args: ["test.js"]
            }
        }
    };

    beforeEach(() => {
        jest.clearAllMocks();
    });

    it('should fetch and display MCP configuration on mount', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: async () => mockConfig
        });

        render(<McpConfigManager />);

        // 初期状態で Loading が表示される
        expect(screen.getByRole('heading', { name: /MCP Architecture/i })).toBeInTheDocument();

        // 読み込み完了後、設定が textarea に反映される
        const textarea = await screen.findByRole('textbox') as HTMLTextAreaElement;
        expect(textarea).toBeInTheDocument();
        expect(JSON.parse(textarea.value)).toEqual(mockConfig);
        expect(authenticatedFetch).toHaveBeenCalledWith('http://localhost/api/skills/mcp/config');
    });

    it('should save configuration successfully when clicking save button', async () => {
        (authenticatedFetch as jest.Mock)
            .mockResolvedValueOnce({
                ok: true,
                json: async () => mockConfig
            })
            .mockResolvedValueOnce({
                ok: true
            });

        render(<McpConfigManager />);

        const textarea = await screen.findByRole('textbox') as HTMLTextAreaElement;
        const saveButton = screen.getByRole('button', { name: /Save & Sync Tools/i });

        // テキストを変更して保存
        const newConfig = { ...mockConfig, mcp_servers: {} };
        fireEvent.change(textarea, { target: { value: JSON.stringify(newConfig) } });
        fireEvent.click(saveButton);

        // 成功メッセージが表示されることを確認
        await waitFor(() => {
            expect(screen.getByText(/Reloaded successfully/i)).toBeInTheDocument();
        });

        expect(authenticatedFetch).toHaveBeenNthCalledWith(2, 'http://localhost/api/skills/mcp/config', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(newConfig)
        });
    });

    it('should show error when backend fails to save configuration', async () => {
        (authenticatedFetch as jest.Mock)
            .mockResolvedValueOnce({
                ok: true,
                json: async () => mockConfig
            })
            .mockResolvedValueOnce({
                ok: false
            });

        render(<McpConfigManager />);

        const saveButton = await screen.findByRole('button', { name: /Save & Sync Tools/i });
        fireEvent.click(saveButton);

        await waitFor(() => {
            expect(screen.getByText(/Error saving/i)).toBeInTheDocument();
        });
    });

    it('should show validation error when textarea has invalid JSON', async () => {
        (authenticatedFetch as jest.Mock).mockResolvedValue({
            ok: true,
            json: async () => mockConfig
        });

        render(<McpConfigManager />);

        const textarea = await screen.findByRole('textbox');
        const saveButton = screen.getByRole('button', { name: /Save & Sync Tools/i });

        // 不正なJSONを入力
        fireEvent.change(textarea, { target: { value: '{ invalid_json: ' } });
        fireEvent.click(saveButton);

        await waitFor(() => {
            expect(screen.getByText(/Invalid JSON or network error/i)).toBeInTheDocument();
        });

        // APIは呼ばれないはず
        expect(authenticatedFetch).toHaveBeenCalledTimes(1); // 初期fetchのみ
    });
});
