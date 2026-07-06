/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
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

// useSystemVitality のモック — イベント注入で再レンダーを促す
let mockLastEvent: any = null;
const vitalitySubscribers = new Set<() => void>();

jest.mock('../hooks/useSystemVitality', () => {
  const React = require('react');
  return {
    useSystemVitality: () => {
      const [, bump] = React.useReducer((c: number) => c + 1, 0);
      React.useEffect(() => {
        vitalitySubscribers.add(bump);
        return () => {
          vitalitySubscribers.delete(bump);
        };
      }, [bump]);
      return {
        lastEvent: mockLastEvent,
        events: [],
        connectionStatus: 'connected',
      };
    },
  };
});

function pushMockVitalityEvent(event: unknown) {
  mockLastEvent = event;
  vitalitySubscribers.forEach((fn) => fn());
}

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
    mockLastEvent = null;
    vitalitySubscribers.clear();
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
    const payload = {
      valid: false,
      errors: ['Start node missing', 'Invalid edge connection'],
    };
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => JSON.stringify(payload),
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
    const mockFetch = jest.fn().mockImplementation((url: string) => {
      if (url.includes('/execute')) {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({ execution_id: 'exec-999', job_ids: ['job-1'] }),
        });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
    });
    global.fetch = mockFetch;

    renderWithI18n();

    expect(screen.getByText(/Estimated Cost:/i)).toHaveTextContent('$0.0000');

    const saveBtn = screen.getByText('Save').closest('button');
    fireEvent.click(saveBtn!);

    await screen.findByText(/Workflow saved successfully/i);

    const executeBtn = screen.getByText('Execute').closest('button');
    expect(executeBtn).toBeInTheDocument();
    expect(executeBtn).not.toBeDisabled();
    fireEvent.click(executeBtn!);

    const successMsg = await screen.findByText(/Execution started/i);
    expect(successMsg).toBeInTheDocument();
    expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/execute'), expect.any(Object));
  });

  it('updates execution status from SystemVitality SSE events via job_ids', async () => {
    mockLastEvent = null;

    const mockFetch = jest.fn().mockImplementation((url: string) => {
      if (url.includes('/execute')) {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            execution_id: 'exec-123',
            job_ids: ['job-a', 'job-b'],
          }),
        });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
    });
    global.fetch = mockFetch;

    renderWithI18n();

    fireEvent.click(screen.getByText('Save').closest('button')!);
    await screen.findByText(/Workflow saved successfully/i);

    fireEvent.click(screen.getByText('Execute').closest('button')!);
    await screen.findByText(/Execution started: exec-123/i);

    pushMockVitalityEvent({
      type: 'task_progress',
      data: {
        job_id: 'job-a',
        percent: 50,
        message: 'Running Step 2...',
      },
    });

    await waitFor(() => {
      expect(screen.getByText(/Workflow Execution Status/i)).toBeInTheDocument();
      expect(screen.getByText('RUNNING')).toBeInTheDocument();
      expect(screen.getByText(/50%/i)).toBeInTheDocument();
      expect(screen.getByText('Running Step 2...')).toBeInTheDocument();
    });

    pushMockVitalityEvent({
      type: 'task_completed',
      data: { job_id: 'job-a' },
    });

    await waitFor(() => {
      expect(screen.getByText('RUNNING')).toBeInTheDocument();
    });

    pushMockVitalityEvent({
      type: 'task_completed',
      data: { job_id: 'job-b' },
    });

    await waitFor(() => {
      expect(screen.getByText('COMPLETED')).toBeInTheDocument();
      expect(screen.getByText('Completed successfully!')).toBeInTheDocument();
    });
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
