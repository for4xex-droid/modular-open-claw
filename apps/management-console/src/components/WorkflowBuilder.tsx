/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
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
import { useWorkflowApi, WorkflowListItem } from '../hooks/useWorkflowApi';
import { estimateCost, fromWorkflowDefinition, NodeType } from '../lib/workflowConverter';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { useTranslation } from '../i18n';

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

const JSON_CONFIG_TYPES = new Set([
  'Start',
  'Transform',
  'HumanApproval',
  'Loop',
  'Parallel',
  'SubWorkflow',
]);

function ConditionNode({ data }: { data: { label?: string } }) {
  return (
    <div className="custom-node condition-node">
      <div className="node-label">{data.label}</div>
      <Handle type="target" position={Position.Top} id="handle-in" />
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

interface NodeConfigDetails {
  trigger?: string;
  model?: string;
  temperature?: number;
  delay_seconds?: number;
  language?: string;
  code?: string;
  mode?: string;
  expression?: string;
  max_iterations?: number;
  server_name?: string;
  tool_name?: string;
  method?: string;
  url_template?: string;
}

interface WorkflowMeta {
  id: string;
  name: string;
  description: string;
  version: number;
}

interface WorkflowTaskEventData {
  job_id?: string;
  percent?: number;
  message?: string;
  error?: string;
}

const detailStr = (v: unknown, fallback = ''): string =>
  typeof v === 'string' ? v : fallback;

const detailNum = (v: unknown, fallback: number): number =>
  typeof v === 'number' && !Number.isNaN(v) ? v : fallback;

function createStartNode(t: (key: string) => string): Node {
  return {
    id: 'start-1',
    type: 'default',
    data: {
      label: t('workflowBuilder.palette.nodes.Start.label'),
      node_type: { Start: { trigger: 'Manual' } },
    },
    position: { x: 250, y: 100 },
  };
}

function newWorkflowMeta(t: (key: string) => string): WorkflowMeta {
  return {
    id: crypto.randomUUID(),
    name: t('workflowBuilder.meta.untitled'),
    description: '',
    version: 1,
  };
}

export default function WorkflowBuilder() {
  const { t } = useTranslation();
  const initialNodes = useMemo(() => [createStartNode(t)], [t]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [workflowMeta, setWorkflowMeta] = useState<WorkflowMeta>(() => newWorkflowMeta(t));
  const [isPersisted, setIsPersisted] = useState(false);
  const [showListModal, setShowListModal] = useState(false);
  const [workflowList, setWorkflowList] = useState<WorkflowListItem[]>([]);
  const [jsonConfigDraft, setJsonConfigDraft] = useState('');
  const [jsonConfigError, setJsonConfigError] = useState<string | null>(null);

  const {
    validateWorkflow,
    saveWorkflow,
    updateWorkflow,
    listWorkflows,
    loadWorkflow,
    executeWorkflow,
    loading,
    error: apiError,
  } = useWorkflowApi();

  const [validationErrors, setValidationErrors] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const { lastEvent } = useSystemVitality();
  const [currentExecutionId, setCurrentExecutionId] = useState<string | null>(null);
  const [trackedJobIds, setTrackedJobIds] = useState<Set<string>>(new Set());
  const [, setCompletedJobIds] = useState<Set<string>>(new Set());
  const [executionStatus, setExecutionStatus] = useState<'idle' | 'running' | 'completed' | 'failed'>('idle');
  const [executionProgress, setExecutionProgress] = useState<number>(0);
  const [executionMessage, setExecutionMessage] = useState<string>('');

  const workflowParams = useCallback(
    () => ({
      id: workflowMeta.id,
      name: workflowMeta.name,
      description: workflowMeta.description,
      version: workflowMeta.version,
      nodes,
      edges,
    }),
    [workflowMeta, nodes, edges]
  );

  useEffect(() => {
    if (!lastEvent || trackedJobIds.size === 0) return;

    const { type, data } = lastEvent;
    const eventData = data as WorkflowTaskEventData;
    const jobId = eventData?.job_id;
    if (!jobId || !trackedJobIds.has(jobId)) return;

    if (type === 'task_progress') {
      setExecutionStatus('running');
      setExecutionProgress(eventData.percent || 0);
      setExecutionMessage(eventData.message || t('workflowBuilder.status.processing'));
    } else if (type === 'task_completed') {
      setCompletedJobIds((prev) => {
        const next = new Set(prev);
        next.add(jobId);
        if (next.size >= trackedJobIds.size) {
          setExecutionStatus('completed');
          setExecutionProgress(100);
          setExecutionMessage(t('workflowBuilder.status.completed'));
        }
        return next;
      });
    } else if (type === 'task_failed') {
      setExecutionStatus('failed');
      setExecutionMessage(eventData.error || t('workflowBuilder.status.failed'));
    }
  }, [lastEvent, trackedJobIds, t]);

  const handleValidate = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    const result = await validateWorkflow(workflowParams());
    if (result.valid) {
      setSuccessMessage(t('workflowBuilder.toolbar.valid'));
    } else {
      setValidationErrors(result.errors || [t('workflowBuilder.validationErrors.unknown')]);
    }
  };

  const handleSave = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    const params = workflowParams();
    const success = isPersisted
      ? await updateWorkflow({ ...params, version: workflowMeta.version + 1 })
      : await saveWorkflow(params);

    if (success) {
      if (isPersisted) {
        setWorkflowMeta((m) => ({ ...m, version: m.version + 1 }));
      } else {
        setIsPersisted(true);
      }
      setSuccessMessage(t('workflowBuilder.toolbar.saved'));
    }
  };

  const handleExecute = async () => {
    setSuccessMessage(null);
    setValidationErrors([]);
    setExecutionStatus('idle');
    setExecutionProgress(0);
    setExecutionMessage('');
    setTrackedJobIds(new Set());
    setCompletedJobIds(new Set());

    const result = await executeWorkflow(workflowMeta.id);
    if (result?.execution_id) {
      setCurrentExecutionId(result.execution_id);
      const ids = new Set(result.job_ids);
      setTrackedJobIds(ids);
      setCompletedJobIds(new Set());
      if (ids.size === 0) {
        setExecutionStatus('completed');
        setExecutionProgress(100);
        setExecutionMessage(t('workflowBuilder.status.completed'));
      } else {
        setExecutionStatus('running');
      }
      setSuccessMessage(t('workflowBuilder.toolbar.started', { id: result.execution_id }));
    }
  };

  const handleNewWorkflow = () => {
    setWorkflowMeta(newWorkflowMeta(t));
    setNodes([createStartNode(t)]);
    setEdges([]);
    setSelectedNode(null);
    setIsPersisted(false);
    setValidationErrors([]);
    setSuccessMessage(null);
    setShowListModal(false);
  };

  const handleOpenList = async () => {
    const list = await listWorkflows();
    setWorkflowList(list);
    setShowListModal(true);
  };

  const handleLoadWorkflowById = async (id: string) => {
    const def = await loadWorkflow(id);
    if (!def) return;

    const { nodes: loadedNodes, edges: loadedEdges } = fromWorkflowDefinition(def);
    setNodes(loadedNodes);
    setEdges(loadedEdges);
    setWorkflowMeta({
      id: def.id,
      name: def.name,
      description: def.description,
      version: def.version,
    });
    setIsPersisted(true);
    setSelectedNode(null);
    setShowListModal(false);
    setSuccessMessage(null);
    setValidationErrors([]);
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

    let node_type: NodeType = { Start: { trigger: 'Manual' } };
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
      position: { x: 250 + nodes.length * 30, y: 150 + nodes.length * 30 },
    };
    setNodes((nds) => [...nds, newNode]);
  };

  const updateNodeLabel = (label: string) => {
    if (!selectedNode) return;
    setNodes((nds) =>
      nds.map((node) =>
        node.id === selectedNode.id
          ? { ...node, data: { ...node.data, label } }
          : node
      )
    );
    setSelectedNode((prev) => (prev ? { ...prev, data: { ...prev.data, label } } : null));
  };

  const getNodeTypeInfo = (node: Node): { typeName: string; details: NodeConfigDetails } => {
    const nodeType = node.data?.node_type as NodeType | undefined;
    if (!nodeType) return { typeName: 'Start', details: { trigger: 'Manual' } };
    const typeName = Object.keys(nodeType)[0];
    const details = Object.values(nodeType)[0] ?? {};
    return { typeName, details: details as NodeConfigDetails };
  };

  const updateNodeTypeDetails = (updates: Record<string, unknown>) => {
    if (!selectedNode) return;
    const { typeName, details } = getNodeTypeInfo(selectedNode);
    const updatedNodeType = {
      [typeName]: {
        ...details,
        ...updates,
      },
    };
    setNodes((nds) =>
      nds.map((node) =>
        node.id === selectedNode.id
          ? { ...node, data: { ...node.data, node_type: updatedNodeType } }
          : node
      )
    );
    setSelectedNode((prev) =>
      prev ? { ...prev, data: { ...prev.data, node_type: updatedNodeType } } : null
    );
  };

  const { typeName, details } = selectedNode
    ? getNodeTypeInfo(selectedNode)
    : { typeName: '', details: {} as NodeConfigDetails };

  useEffect(() => {
    if (!selectedNode || !JSON_CONFIG_TYPES.has(typeName)) {
      setJsonConfigDraft('');
      setJsonConfigError(null);
      return;
    }
    const { details: draftDetails } = getNodeTypeInfo(selectedNode);
    setJsonConfigDraft(JSON.stringify(draftDetails, null, 2));
    setJsonConfigError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNode?.id, typeName]);

  const applyJsonConfig = (raw: string) => {
    setJsonConfigDraft(raw);
    try {
      const parsed = JSON.parse(raw);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        throw new Error('invalid');
      }
      updateNodeTypeDetails(parsed as Record<string, unknown>);
      setJsonConfigError(null);
    } catch {
      setJsonConfigError(t('workflowBuilder.config.jsonInvalid'));
    }
  };

  return (
    <div className="workflow-builder-container">
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

      <div className="workflow-canvas-wrapper">
        <div className="workflow-toolbar">
          <div className="workflow-meta-fields">
            <input
              type="text"
              className="workflow-meta-name"
              value={workflowMeta.name}
              onChange={(e) => setWorkflowMeta((m) => ({ ...m, name: e.target.value }))}
              placeholder={t('workflowBuilder.meta.name')}
              aria-label={t('workflowBuilder.meta.name')}
            />
            <span className="workflow-meta-version">
              {t('workflowBuilder.meta.version', { version: workflowMeta.version })}
            </span>
          </div>
          <div className="workflow-toolbar-actions">
            <button type="button" onClick={handleNewWorkflow} disabled={loading} className="btn-secondary">
              {t('workflowBuilder.meta.new')}
            </button>
            <button type="button" onClick={handleOpenList} disabled={loading} className="btn-secondary">
              {t('workflowBuilder.meta.open')}
            </button>
            <button type="button" onClick={handleValidate} disabled={loading} className="btn-secondary">
              {t('workflowBuilder.toolbar.validate')}
            </button>
            <button type="button" onClick={handleSave} disabled={loading} className="btn-secondary">
              {t('workflowBuilder.toolbar.save')}
            </button>
            <button type="button" onClick={handleExecute} disabled={loading || !isPersisted} className="btn-primary">
              {t('workflowBuilder.toolbar.execute')}
            </button>
          </div>
          <span className="toolbar-cost">
            {t('workflowBuilder.toolbar.estimatedCost')}: <strong>${cost.estimatedUsd.toFixed(4)}</strong>{' '}
            ({t('workflowBuilder.toolbar.nodesCount', { count: cost.nodes })})
          </span>
          {loading && <span className="toolbar-loading">{t('workflowBuilder.toolbar.processing')}</span>}
          {successMessage && <span className="toolbar-success">{successMessage}</span>}
          {apiError && (
            <span className="toolbar-error">
              {t('workflowBuilder.toolbar.error')}: {apiError}
            </span>
          )}
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
              <span>
                {t('workflowBuilder.status.label')}: <strong>{executionStatus.toUpperCase()}</strong>
              </span>
              {currentExecutionId && <span>ID: {currentExecutionId}</span>}
              {executionStatus === 'running' && (
                <span>{t('workflowBuilder.status.progress', { percent: executionProgress })}</span>
              )}
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

      {showListModal && (
        <div className="workflow-list-modal-backdrop" onClick={() => setShowListModal(false)}>
          <div className="workflow-list-modal" onClick={(e) => e.stopPropagation()}>
            <h3>{t('workflowBuilder.meta.listTitle')}</h3>
            {workflowList.length === 0 ? (
              <p className="workflow-list-empty">{t('workflowBuilder.meta.listEmpty')}</p>
            ) : (
              <ul className="workflow-list">
                {workflowList.map((wf) => (
                  <li key={wf.id}>
                    <div className="workflow-list-item-info">
                      <strong>{wf.name}</strong>
                      <span>{wf.description || wf.id}</span>
                    </div>
                    <button type="button" className="btn-secondary" onClick={() => handleLoadWorkflowById(wf.id)}>
                      {t('workflowBuilder.meta.load')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <button type="button" className="btn-close" onClick={() => setShowListModal(false)}>
              {t('workflowBuilder.config.close')}
            </button>
          </div>
        </div>
      )}

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

            {typeName === 'LlmPrompt' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.llmModel')}</label>
                  <input
                    type="text"
                    value={detailStr(details.model)}
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
                    value={detailNum(details.temperature, 0.7)}
                    onChange={(e) => updateNodeTypeDetails({ temperature: parseFloat(e.target.value) || 0 })}
                  />
                </div>
              </>
            )}

            {typeName === 'McpToolCall' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.mcpServer')}</label>
                  <input
                    type="text"
                    value={detailStr(details.server_name)}
                    onChange={(e) => updateNodeTypeDetails({ server_name: e.target.value })}
                  />
                </div>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.toolName')}</label>
                  <input
                    type="text"
                    value={detailStr(details.tool_name)}
                    onChange={(e) => updateNodeTypeDetails({ tool_name: e.target.value })}
                  />
                </div>
              </>
            )}

            {typeName === 'HttpRequest' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.httpMethod')}</label>
                  <select
                    value={detailStr(details.method, 'GET')}
                    onChange={(e) => updateNodeTypeDetails({ method: e.target.value })}
                  >
                    <option value="GET">GET</option>
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="PATCH">PATCH</option>
                    <option value="DELETE">DELETE</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.urlTemplate')}</label>
                  <input
                    type="text"
                    value={detailStr(details.url_template)}
                    onChange={(e) => updateNodeTypeDetails({ url_template: e.target.value })}
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
                  value={detailNum(details.delay_seconds, 60)}
                  onChange={(e) => updateNodeTypeDetails({ delay_seconds: parseInt(e.target.value, 10) || 1 })}
                />
              </div>
            )}

            {typeName === 'WasmCode' && (
              <>
                <div className="form-group">
                  <label>{t('workflowBuilder.config.language')}</label>
                  <select
                    value={detailStr(details.language, 'javascript')}
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
                    value={detailStr(details.code)}
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
                    value={detailStr(details.mode, 'Expression')}
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
                    value={detailStr(details.expression)}
                    onChange={(e) => updateNodeTypeDetails({ expression: e.target.value })}
                  />
                </div>
              </>
            )}

            {JSON_CONFIG_TYPES.has(typeName) && (
              <div className="form-group">
                <label>{t('workflowBuilder.config.jsonConfig')}</label>
                <textarea
                  className="json-config-editor"
                  value={jsonConfigDraft}
                  onChange={(e) => applyJsonConfig(e.target.value)}
                  rows={8}
                />
                {jsonConfigError && <span className="json-config-error">{jsonConfigError}</span>}
              </div>
            )}

            <button type="button" className="btn-close" onClick={() => setSelectedNode(null)}>
              {t('workflowBuilder.config.close')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
