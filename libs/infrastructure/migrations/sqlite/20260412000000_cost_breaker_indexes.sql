-- Performance indexing for Cost Circuit Breaker
CREATE INDEX IF NOT EXISTS idx_resource_usage_logs_created_at ON resource_usage_logs(created_at);
