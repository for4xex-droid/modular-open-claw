import React, { useState, useCallback, useEffect, useMemo } from 'react';
import {
  ReactFlow,
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  Handle,
  Position,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import './WorkflowBuilder.css';
import { useWorkflowApi } from '../hooks/useWorkflowApi';
import { estimateCost } from '../lib/workflowConverter';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { useTranslation } from '../i18n';

// 12種類のノード種別 — ラベルと説明は i18n キーで解決
const PALETTE_NODES = [
  { type: 'Start' },
  { type: 'LlmPrompt' },
  { type: 'McpToolCall' },
  { type: 'HttpRequest' },
  { type: 'Transform' },
  { type: 'Condition' },
  { type: 'Timer' },
  { type: 'WasmCode' },
  { type: 'HumanApproval' },
  { type: 'Loop' },
  { type: 'Parallel' },
  { type: 'SubWorkflow' },
];

// Conditionノード用のカスタムコンポーネント (2-Handle出力)
function ConditionNode({ data }: { data: any }) {
  return (
    <div className="custom-node condition-node">
      <div className="node-label">{data.label}</div>
      {/* 入力ハンドル */}
      <Handle type="target" position={Position.Top} id="handle-in" />
      {/* True / False 出力ハンドル */}
      <div className="condition-handles">
        <div className="handle-wrapper true-handle">
          <span>True</span>
          <Handle type="source" position={Position.Bottom} id="handle-true" />
        </div>
        <div className="handle-wrapper false-handle">
          <span>False</span>
          <Handle type="source" position={Position.Bottom} id="handle-false" />
        </div>
      </div>
    </div>
  );
}

const nodeTypes = {
  Condition: ConditionNode,
};

export default function WorkflowBuilder() {
  const { t } = useTranslation();

  const initialNodes: Node[] = useMemo(() => [
    {
      id: 'start-1',
      type: 'default', // テスト簡略化のため default ノードを利用
      data: { label: t('workflowBuilder.palette.nodes.Start.label'), node_type: { Start: { trigger: 'Manual' } } },
      position: { x: 250, y: 100 },
    },
  ], [t]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);

  const { validateWorkflow, saveWorkflow, executeWorkflow, loading, error: apiError } = useWorkflowApi();
  const [validationErrors, setValidationErrors] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const { lastEvent } = useSystemVitality();
  const [currentExecutionId, setCurrentExecutionId] = useState<string | null>(null);
  const [executionStatus, setExecutionStatus] = useState<'idle' | 'running' | 'completed' | 'failed'>('idle');
  const [executionProgress, setExecutionProgress] = useState<number>(0);
  const [executionMessage, setExecutionMessage] = useState<string>('');

  useEffect(() => {
    if (!lastEvent || !currentExecutionId) return;

    const { type, data } = lastEvent;
    const eventData = data as any;

    if (eventData?.job_id !== currentExecutionId) return;

    if (type === 'task_progress') {
      setExecutionStatus('running');
      setExecutionProgress(eventData.percent || 0);
      setExecutionMessage(eventData.message || 'Processing...');
    } else if (type === 'task_completed') {
      setExecutionStatus('completed');
      setExecutionProgress(100);
      setExecutionMessage(t('workflowBuilder.status.completed'));
    } else if (type === 'task_failed') {
      setExecutionStatus('failed');
      setExecutionMessage(eventData.error || 'Failed');
    }
  }, [lastEvent, currentExecutionId, t]);

  const handleValidate = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    const result = await validateWorkflow({
      id: '8437dfb3-c4e2-4da6-bb4a-262de6e1099c',
      name: 'My Workflow',
      description: '',
      version: 1,
      nodes,
      edges,
    });
    if (result.valid) {
      setSuccessMessage(t('workflowBuilder.toolbar.valid'));
    } else {
      setValidationErrors(result.errors || ['Validation failed with unknown error']);
    }
  };

  const handleSave = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    const success = await saveWorkflow({
      id: '8437dfb3-c4e2-4da6-bb4a-262de6e1099c',
      name: 'My Workflow',
      description: '',
      version: 1,
      nodes,
      edges,
    });
    if (success) {
      setSuccessMessage(t('workflowBuilder.toolbar.saved'));
    }
  };

  const handleExecute = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    setExecutionStatus('idle');
    setExecutionProgress(0);
    setExecutionMessage('');

    const result = await executeWorkflow('8437dfb3-c4e2-4da6-bb4a-262de6e1099c');
    if (result && result.execution_id) {
      setCurrentExecutionId(result.execution_id);
      setExecutionStatus('running');
      setSuccessMessage(t('workflowBuilder.toolbar.started', { id: result.execution_id }));
    }
  };

  const cost = estimateCost(nodes);

  const onConnect = useCallback(
    (params: Connection | Edge) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  const addNode = (type: string, labelKey: string) => {
    const id = `${type.toLowerCase()}-${nodes.length + 1}`;
    
    // 初期 node_type のマッピングを定義
    let node_type: any = { Start: { trigger: 'Manual' } };
    if (type === 'LlmPrompt') {
      node_type = { LlmPrompt: { model: 'gemini-1.5-flash', temperature: 0.7 } };
    } else if (type === 'McpToolCall') {
      node_type = { McpToolCall: { server_name: '', tool_name: '' } };
    } else if (type === 'HttpRequest') {
      node_type = { HttpRequest: { method: 'GET', url_template: '' } };
    } else if (type === 'Transform') {
      node_type = { Transform: { expression: '' } };
    } else if (type === 'Condition') {
      node_type = { Condition: { expression: '', mode: 'Expression' } };
    } else if (type === 'Timer') {
      node_type = { Timer: { delay_seconds: 60 } };
    } else if (type === 'WasmCode') {
      node_type = { WasmCode: { code: '', language: 'javascript' } };
    } else if (type === 'HumanApproval') {
      node_type = { HumanApproval: { prompt_message: '' } };
    } else if (type === 'Loop') {
      node_type = { Loop: { iterator_expression: '', max_iterations: 10 } };
    } else if (type === 'Parallel') {
      node_type = { Parallel: { wait_mode: 'All' } };
    } else if (type === 'SubWorkflow') {
      node_type = { SubWorkflow: { workflow_id: '00000000-0000-0000-0000-000000000000' } };
    }

    const newNode: Node = {
      id,
      type: type === 'Condition' ? 'Condition' : 'default',
      data: { label: t(labelKey), node_type },
      position: { x: 250 + (nodes.length * 30), y: 150 + (nodes.length * 30) },
    };
    setNodes((nds) => [...nds, newNode]);
  };

  const updateNodeLabel = (label: string) => {
    if (!selectedNode) return;
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === selectedNode.id) {
          return {
            ...node,
            data: { ...node.data, label },
          };
        }
        return node;
      })
    );
    setSelectedNode((prev) => prev ? { ...prev, data: { ...prev.data, label } } : null);
  };

  const getNodeTypeInfo = (node: Node) => {
    const nodeType = node.data?.node_type as any;
    if (!nodeType) return { typeName: 'Start', details: { trigger: 'Manual' } };
    const typeName = Object.keys(nodeType)[0];
    const details = nodeType[typeName] || {};
    return { typeName, details };
  };

  const updateNodeTypeDetails = (updates: any) => {
    if (!selectedNode) return;
    const { typeName, details } = getNodeTypeInfo(selectedNode);
    const updatedNodeType = {
      [typeName]: {
        ...details,
        ...updates,
      },
    };
    setNodes((nds) =>
      nds.map((node) => {
        if (node.id === selectedNode.id) {
          return {
            ...node,
            data: { ...node.data, node_type: updatedNodeType },
          };
        }
        return node;
      })
    );
    setSelectedNode((prev) =>
      prev
        ? {
            ...prev,
            data: { ...prev.data, node_type: updatedNodeType },
          }
        : null
    );
  };

  const { typeName, details } = selectedNode ? getNodeTypeInfo(selectedNode) : { typeName: '', details: {} as any };

  return (
    <div className="workflow-builder-container">
      {/* 左パレット */}
      <div className="workflow-palette">
        <h3>{t('workflowBuilder.palette.title')}</h3>
        <p className="palette-info">{t('workflowBuilder.palette.info')}</p>
        <div className="palette-list">
          {PALETTE_NODES.map((item) => (
            <button
              key={item.type}
              className="palette-item"
              onClick={() => addNode(item.type, `workflowBuilder.palette.nodes.${item.type}.label`)}
            >
              <strong>{t(`workflowBuilder.palette.nodes.${item.type}.label`)}</strong>
              <span>{t(`workflowBuilder.palette.nodes.${item.type}.desc`)}</span>
            </button>
          ))}
        </div>
      </div>

      {/* キャンバス */}
      <div className="workflow-canvas-wrapper">
        <div className="workflow-toolbar">
          <button onClick={handleValidate} disabled={loading} className="btn-secondary">
            {t('workflowBuilder.toolbar.validate')}
          </button>
          <button onClick={handleSave} disabled={loading} className="btn-secondary">
            {t('workflowBuilder.toolbar.save')}
          </button>
          <button onClick={handleExecute} disabled={loading} className="btn-primary">
            {t('workflowBuilder.toolbar.execute')}
          </button>
          <span className="toolbar-cost">
            {t('workflowBuilder.toolbar.estimatedCost')}: <strong>${cost.estimatedUsd.toFixed(4)}</strong> ({t('workflowBuilder.toolbar.nodesCount', { count: cost.nodes })})
          </span>
          {loading && <span className="toolbar-loading">{t('workflowBuilder.toolbar.processing')}</span>}
          {successMessage && <span className="toolbar-success">{successMessage}</span>}
          {apiError && <span className="toolbar-error">{t('workflowBuilder.toolbar.error')}: {apiError}</span>}
        </div>

        {validationErrors.length > 0 && (
          <div className="validation-error-banner">
            <h4>{t('workflowBuilder.validationErrors.title')}</h4>
            <ul>
              {validationErrors.map((err, idx) => (
                <li key={idx}>{err}</li>
              ))}
            </ul>
          </div>
        )}

        {executionStatus !== 'idle' && (
          <div className={`execution-status-panel status-${executionStatus}`}>
            <h4>{t('workflowBuilder.status.title')}</h4>
            <div className="status-meta">
              <span>{t('workflowBuilder.status.label')}: <strong>{executionStatus.toUpperCase()}</strong></span>
              {executionStatus === 'running' && <span>{t('workflowBuilder.status.progress', { percent: executionProgress })}</span>}
            </div>
            {executionMessage && <p className="status-msg">{executionMessage}</p>}
          </div>
        )}

        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodeClick={onNodeClick}
          fitView
        >
          <Background color="var(--border-glass-bright)" gap={16} />
          <Controls />
          <MiniMap
            nodeStrokeColor={() => 'var(--accent-cyan)'}
            nodeColor={() => 'var(--bg-glass-heavy)'}
            maskColor="var(--bg-deep-glass)"
          />
        </ReactFlow>
      </div>

      {/* 右設定パネル */}
      {selectedNode && (
        <div className="workflow-config-panel">
          <h3>{t('workflowBuilder.config.title')}</h3>
          <div className="config-form">
            <div className="form-group">
              <label>{t('workflowBuilder.config.nodeId')}</label>
              <input type="text" value={selectedNode.id} disabled className="input-disabled" />
            </div>
            <div className="form-group">
              <label>{t('workflowBuilder.config.label')}</label>
              <input
                type="text"
                value={selectedNode.data.label as string}
                onChange={(e) => updateNodeLabel(e.target.value)}
              />
            </div>

            {/* ノードタイプ固有の編集フォーム */}
            {typeName === 'LlmPrompt' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.llmModel')}</label>
                  <input
                    type="text"
                    value={details.model || ''}
                    onChange={(e) => updateNodeTypeDetails({ model: e.target.value })}
                  />
                </div>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.temperature')}</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    value={details.temperature ?? 0.7}
                    onChange={(e) => updateNodeTypeDetails({ temperature: parseFloat(e.target.value) || 0 })}
                  />
                </div>
              </>
            )}

            {typeName === 'Timer' && (
              <div className="form-group">
                <label>{t('workflowBuilder.config.delaySeconds')}</label>
                <input
                  type="number"
                  min="1"
                  value={details.delay_seconds || 60}
                  onChange={(e) => updateNodeTypeDetails({ delay_seconds: parseInt(e.target.value) || 1 })}
                />
              </div>
            )}

            {typeName === 'WasmCode' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.language')}</label>
                  <select
                    value={details.language || 'javascript'}
                    onChange={(e) => updateNodeTypeDetails({ language: e.target.value })}
                  >
                    <option value="javascript">JavaScript</option>
                    <option value="typescript">TypeScript</option>
                    <option value="rust">Rust</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.wasmCode')}</label>
                  <textarea
                    value={details.code || ''}
                    onChange={(e) => updateNodeTypeDetails({ code: e.target.value })}
                    rows={4}
                  />
                </div>
              </>
            )}

            {typeName === 'Condition' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.evaluationMode')}</label>
                  <select
                    value={details.mode || 'Expression'}
                    onChange={(e) => updateNodeTypeDetails({ mode: e.target.value })}
                  >
                    <option value="Expression">Expression</option>
                    <option value="LlmJudge">LLM Judge</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.expression')}</label>
                  <input
                    type="text"
                    value={details.expression || ''}
                    onChange={(e) => updateNodeTypeDetails({ expression: e.target.value })}
                  />
                </div>
              </>
            )}

            <button className="btn-close" onClick={() => setSelectedNode(null)}>
              {t('workflowBuilder.config.close')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
