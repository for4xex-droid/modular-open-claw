-- Project NURTURE: Migrate conversion_rate from REAL to BIGINT (bps)
-- 2026-04-16

CREATE TABLE nurture_points_new (
    actor_id TEXT PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 0,
    lifetime_earned BIGINT NOT NULL DEFAULT 0,
    lifetime_withdrawn BIGINT NOT NULL DEFAULT 0,
    conversion_rate BIGINT NOT NULL DEFAULT 10000
);

INSERT INTO nurture_points_new (actor_id, balance, lifetime_earned, lifetime_withdrawn, conversion_rate)
SELECT actor_id, balance, lifetime_earned, lifetime_withdrawn, CAST(conversion_rate * 10000 AS BIGINT) FROM nurture_points;

DROP TABLE nurture_points;

ALTER TABLE nurture_points_new RENAME TO nurture_points;
