/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import WorkflowBuilder from './WorkflowBuilder';
import { LanguageProvider } from '../i18n';

// React Flow v12 のモック
// Jest のテスト環境下で Canvas / ResizeObserver のエラーを防ぐため
jest.mock('@xyflow/react', () => {
  const React = require('react');
  return {
    ReactFlow: ({ children, nodes, nodeTypes, onNodesChange, edges, onEdgesChange, onConnect, onNodeClick }: any) => (
      <div data-testid="rf-canvas">
        <div data-testid="nodes-count">{nodes.length}</div>
        <div data-testid="rf-nodes">
          {nodes.map((node: any) => {
            const NodeTypeComp = nodeTypes ? nodeTypes[node.type] : null;
            return (
              <div key={node.id} data-testid={`node-wrapper-${node.id}`}>
                <button
                  data-testid={`node-item-${node.id}`}
                  onClick={(e) => onNodeClick && onNodeClick(e, node)}
                >
                  {node.data?.label || node.id}
                </button>
                {NodeTypeComp && <NodeTypeComp id={node.id} data={node.data} />}
              </div>
            );
          })}
        </div>
        {children}
      </div>
    ),
    Controls: () => <div data-testid="rf-controls" />,
    Background: () => <div data-testid="rf-background" />,
    MiniMap: () => <div data-testid="rf-minimap" />,
    useNodesState: (initialNodes: any) => {
      const [nodes, setNodes] = React.useState(initialNodes);
      return [nodes, setNodes, jest.fn()];
    },
    useEdgesState: (initialEdges: any) => {
      const [edges, setEdges] = React.useState(initialEdges);
      return [edges, setEdges, jest.fn()];
    },
    addEdge: (connection: any, edges: any) => [...edges, connection],
    Handle: (props: any) => <div data-testid={`rf-handle-${props.id || 'default'}`} data-type={props.type} />,
    Position: { Left: 'left', Right: 'right', Top: 'top', Bottom: 'bottom' },
  };
});

// useSystemVitality のモック
let mockLastEvent: any = null;
jest.mock('../hooks/useSystemVitality', () => ({
  useSystemVitality: () => ({
    lastEvent: mockLastEvent,
    events: [],
    connectionStatus: 'connected',
  }),
}));

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000'
}));

describe('WorkflowBuilder Component', () => {
  const renderWithI18n = (ui?: React.ReactElement, options?: any) => render(
    ui || <WorkflowBuilder />,
    { wrapper: LanguageProvider, ...options }
  );

  beforeEach(() => {
    localStorage.setItem('aiome_lang', 'en');
  });

  it('renders without crashing and shows React Flow canvas', () => {
    renderWithI18n();
    
    // キャンバスと基本コントロールの描画確認
    expect(screen.getByTestId('rf-canvas')).toBeInTheDocument();
    expect(screen.getByTestId('rf-controls')).toBeInTheDocument();
    expect(screen.getByTestId('rf-background')).toBeInTheDocument();
  });

  it('contains node palette and allows dragging nodes', () => {
    const { container } = renderWithI18n();
    const palette = container.querySelector('.workflow-palette');
    expect(palette).toBeInTheDocument();
    
    // パレット内のノードタイトルの確認
    expect(palette).toHaveTextContent('Start Node');
    expect(palette).toHaveTextContent('LLM Prompt');
    expect(palette).toHaveTextContent('HTTP Request');
    expect(palette).toHaveTextContent('Timer');
    expect(palette).toHaveTextContent('WasmCode');
  });

  it('shows config panel when a node is selected', () => {
    renderWithI18n();
    
    // 最初はスタートノードだけが配置されている状態を検証
    expect(screen.getByTestId('nodes-count')).toHaveTextContent('1');
    
    // ノード設定パネルのタイトルがない状態を確認（未選択時）
    expect(screen.queryByText('Node Configuration')).not.toBeInTheDocument();
  });

  it('shows custom config inputs based on node_type when selected', () => {
    renderWithI18n();

    // 「LLM Prompt」を追加
    const llmButton = screen.getByText('LLM Prompt').closest('button');
    fireEvent.click(llmButton!);

    // 追加されたノード「llmprompt-2」をクリック
    const nodeEl = screen.getByTestId('node-item-llmprompt-2');
    fireEvent.click(nodeEl);

    // 設定パネルが表示されることを確認
    expect(screen.getByText('Node Configuration')).toBeInTheDocument();

    // LLM Prompt 固有の入力欄（Model, Temperature）が存在することを確認
    expect(screen.getByText('LLM Model')).toBeInTheDocument();
    expect(screen.getByText('Temperature')).toBeInTheDocument();
  });

  it('displays validation errors when validation fails', async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        valid: false,
        errors: ['Start node missing', 'Invalid edge connection'],
      }),
    });
    global.fetch = mockFetch;

    renderWithI18n();

    // 「Validate」ボタンをクリック
    const validateBtn = screen.getByText('Validate').closest('button');
    expect(validateBtn).toBeInTheDocument();
    fireEvent.click(validateBtn!);

    // エラーが画面に表示されるのを待つ
    const errorEl1 = await screen.findByText('Start node missing');
    const errorEl2 = await screen.findByText('Invalid edge connection');
    expect(errorEl1).toBeInTheDocument();
    expect(errorEl2).toBeInTheDocument();
  });

  it('displays estimated execution cost and triggers execute on button click', async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ execution_id: 'exec-999' }),
    });
    global.fetch = mockFetch;

    renderWithI18n();

    // 初期配置のノードが Start Node 1つなので、コストは $0.0000 になるはず
    expect(screen.getByText(/Estimated Cost:/i)).toHaveTextContent('$0.0000');

    // 「Execute」ボタンをクリック
    const executeBtn = screen.getByText('Execute').closest('button');
    expect(executeBtn).toBeInTheDocument();
    fireEvent.click(executeBtn!);

    // 実行成功メッセージが表示されるのを確認
    const successMsg = await screen.findByText(/Execution started/i);
    expect(successMsg).toBeInTheDocument();
    expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.any(Object));
  });

  it('updates execution status from SystemVitality SSE events', async () => {
    mockLastEvent = null;

    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ execution_id: 'exec-123' }),
    });
    global.fetch = mockFetch;

    const { rerender } = renderWithI18n();

    // 「Execute」ボタンをクリックして execution_id='exec-123' をセットさせる
    const executeBtn = screen.getByText('Execute').closest('button');
    fireEvent.click(executeBtn!);

    // 実行成功メッセージが表示されて execution_id がセットされるのを待つ
    await screen.findByText(/Execution started: exec-123/i);

    // 進捗イベントを注入
    mockLastEvent = {
      type: 'task_progress',
      data: {
        job_id: 'exec-123',
        percent: 50,
        message: 'Running Step 2...',
      },
    };

    rerender(<WorkflowBuilder />);

    // 画面上にステータスと進捗メッセージが描画されたことを確認
    expect(screen.getByText(/Workflow Execution Status/i)).toBeInTheDocument();
    expect(screen.getByText('RUNNING')).toBeInTheDocument();
    expect(screen.getByText(/50%/i)).toBeInTheDocument();
    expect(screen.getByText('Running Step 2...')).toBeInTheDocument();

    // 完了イベントを注入
    mockLastEvent = {
      type: 'task_completed',
      data: {
        job_id: 'exec-123',
        result: 'Success',
      },
    };

    rerender(<WorkflowBuilder />);

    // ステータスが COMPLETED になり、完了メッセージが表示されることを確認
    expect(screen.getByText('COMPLETED')).toBeInTheDocument();
    expect(screen.getByText('Completed successfully!')).toBeInTheDocument();
  });

  it('renders Condition node with true and false output handles', () => {
    renderWithI18n();

    // 「Condition」ノードを追加
    const condButton = screen.getByText('Condition').closest('button');
    fireEvent.click(condButton!);

    // 追加された Condition ノードのラッパー内に、true/false 用の出力ハンドルが存在することを確認
    expect(screen.getByTestId('rf-handle-handle-true')).toBeInTheDocument();
    expect(screen.getByTestId('rf-handle-handle-false')).toBeInTheDocument();
  });
});
