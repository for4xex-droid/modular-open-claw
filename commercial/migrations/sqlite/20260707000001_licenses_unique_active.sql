-- B-2: Prevent duplicate active licenses per owner+asset (refund revokes via revoked_at).
CREATE UNIQUE INDEX IF NOT EXISTS idx_licenses_owner_asset_active
    ON nurture_licenses(owner_id, asset_id)
    WHERE revoked_at IS NULL;
