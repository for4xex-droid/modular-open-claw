-- Create a generic audit function
CREATE OR REPLACE FUNCTION process_audit() RETURNS TRIGGER AS $$
DECLARE
    entity_id TEXT;
    action TEXT;
    payload TEXT;
    prev_hash TEXT;
    new_hash TEXT;
BEGIN
    action := TG_OP;

    IF (TG_OP = 'DELETE') THEN
        IF TG_TABLE_NAME = 'system_state' THEN
            entity_id := OLD.key;
        ELSIF TG_TABLE_NAME = 'gig_deliveries' THEN
            entity_id := OLD.order_id;
        ELSE
            entity_id := OLD.id;
        END IF;
        payload := TG_TABLE_NAME || ':DELETE:' || entity_id;
    ELSIF (TG_OP = 'UPDATE') THEN
        IF TG_TABLE_NAME = 'system_state' THEN
            entity_id := NEW.key;
        ELSIF TG_TABLE_NAME = 'gig_deliveries' THEN
            entity_id := NEW.order_id;
        ELSE
            entity_id := NEW.id;
        END IF;
        payload := TG_TABLE_NAME || ':UPDATE:' || entity_id;
    ELSIF (TG_OP = 'INSERT') THEN
        IF TG_TABLE_NAME = 'system_state' THEN
            entity_id := NEW.key;
        ELSIF TG_TABLE_NAME = 'gig_deliveries' THEN
            entity_id := NEW.order_id;
        ELSE
            entity_id := NEW.id;
        END IF;
        payload := TG_TABLE_NAME || ':INSERT:' || entity_id;
    END IF;

    -- Fetch the previous hash directly from the audit ledger
    SELECT current_hash INTO prev_hash FROM audit_ledger_global ORDER BY id DESC LIMIT 1;
    IF prev_hash IS NULL THEN
        prev_hash := 'GENESIS';
    END IF;

    -- Generate a new hash (using MD5 as a simple placeholder similar to randomblob(16))
    new_hash := md5(random()::text);

    INSERT INTO audit_ledger_global (
        table_name, operation, record_id, new_data, prev_hash, current_hash
    ) VALUES (
        TG_TABLE_NAME, action, COALESCE(entity_id::text, 'UNKNOWN'), payload, prev_hash, new_hash
    );

    IF (TG_OP = 'DELETE') THEN
        RETURN OLD;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Attach triggers to the same tables as SQLite
CREATE TRIGGER audit_trigger_jobs AFTER INSERT OR UPDATE ON jobs FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_karma_logs AFTER INSERT OR UPDATE ON karma_logs FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_system_state AFTER INSERT OR UPDATE ON system_state FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_ai_artifacts AFTER INSERT OR UPDATE ON ai_artifacts FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_revenue_splits AFTER INSERT OR UPDATE ON revenue_splits FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_gig_intents AFTER INSERT OR UPDATE ON gig_intents FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_gig_bids AFTER INSERT OR UPDATE ON gig_bids FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_escrows AFTER INSERT OR UPDATE ON escrows FOR EACH ROW EXECUTE FUNCTION process_audit();
CREATE TRIGGER audit_trigger_gig_deliveries AFTER INSERT OR UPDATE ON gig_deliveries FOR EACH ROW EXECUTE FUNCTION process_audit();

