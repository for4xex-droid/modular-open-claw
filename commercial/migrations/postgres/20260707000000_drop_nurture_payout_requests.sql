-- ADR-052: Remove dead fiat payout request table (never wired to live payout flow).
DROP TABLE IF EXISTS nurture_payout_requests;
