-- GAP-1: Skill Maturity Database (TypeState metadata storage)
-- Records the empirical success rate and maturity state of VerifiedSkills.
CREATE TABLE IF NOT EXISTS skill_maturity (
    skill_name TEXT PRIMARY KEY,
    maturity TEXT NOT NULL,          -- 'Quarantined', 'Probation', 'Trusted', 'Veteran'
    promotion_count INTEGER NOT NULL DEFAULT 0,
    last_promoted_at TEXT NOT NULL
);
