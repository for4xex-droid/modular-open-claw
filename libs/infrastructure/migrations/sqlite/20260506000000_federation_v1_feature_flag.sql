-- Enable Federation v1.0 by default
INSERT OR IGNORE INTO system_settings (key, value, category, is_secret)
VALUES ('feature_flag.federation_v1_5', 'true', 'federation', 0);
