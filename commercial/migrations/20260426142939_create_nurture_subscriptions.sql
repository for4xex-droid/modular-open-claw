CREATE TABLE IF NOT EXISTS nurture_subscriptions (
    id TEXT PRIMARY KEY NOT NULL,
    actor_id TEXT NOT NULL,
    stripe_subscription_id TEXT UNIQUE NOT NULL,
    plan_id TEXT NOT NULL,
    status TEXT NOT NULL,
    current_period_end DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_nurture_subscriptions_actor ON nurture_subscriptions(actor_id);
