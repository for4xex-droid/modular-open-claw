-- Migration: 20260614000000_workflows.sql

CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    creator_id TEXT NOT NULL,          -- エージェント/ユーザー ID
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',    -- JSON 配列: ["automation", "seo", ...]
    visibility TEXT NOT NULL DEFAULT 'private'
        CHECK(visibility IN ('private', 'unlisted', 'community', 'marketplace')),
    current_version INTEGER NOT NULL DEFAULT 1,
    is_template INTEGER NOT NULL DEFAULT 0,  -- テンプレートフラグ
    fork_source_id TEXT,               -- フォーク元のワークフロー ID
    execution_count INTEGER NOT NULL DEFAULT 0,
    last_executed_at TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_workflows_creator ON workflows(creator_id);
CREATE INDEX IF NOT EXISTS idx_workflows_visibility ON workflows(visibility)
    WHERE visibility IN ('community', 'marketplace');
CREATE INDEX IF NOT EXISTS idx_workflows_tags ON workflows(tags);

CREATE TABLE IF NOT EXISTS workflow_versions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    definition TEXT NOT NULL,           -- WorkflowDefinition の JSON (ノード + エッジ + 変数)
    change_summary TEXT NOT NULL DEFAULT '',
    created_at TEXT DEFAULT (datetime('now')),
    FOREIGN KEY(workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    UNIQUE(workflow_id, version)
);

CREATE TABLE IF NOT EXISTS workflow_executions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Running'
        CHECK(status IN ('Running', 'Completed', 'Failed', 'Cancelled')),
    input_variables TEXT NOT NULL DEFAULT '{}',  -- 実行時パラメータ
    output_result TEXT,
    root_job_id TEXT,                   -- 最初に enqueue された Job の ID
    started_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT,
    FOREIGN KEY(workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    FOREIGN KEY(root_job_id) REFERENCES jobs(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_wf_exec_workflow ON workflow_executions(workflow_id);
CREATE INDEX IF NOT EXISTS idx_wf_exec_status ON workflow_executions(status);
