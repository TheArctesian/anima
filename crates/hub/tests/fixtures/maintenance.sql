-- K6 synthetic maintenance windows (contracts/fixtures/phase1.json). Not real hosts.

INSERT INTO maintenance.windows (id, name, kind, timezone, rule) VALUES
    ('0190a000-0000-7000-8004-000000000001', 'Sunday updates', 'recurring_weekday', 'Pacific/Auckland',
     '{"kind": "recurring_weekday", "weekdays": ["sun"], "startTime": "03:00:00", "durationMinutes": 60}'),
    ('0190a000-0000-7000-8004-000000000002', 'Overnight backup', 'recurring_weekday', 'Pacific/Auckland',
     '{"kind": "recurring_weekday", "weekdays": ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], "startTime": "23:30:00", "durationMinutes": 90}');

INSERT INTO maintenance.window_targets (window_id, target_type, target) VALUES
    ('0190a000-0000-7000-8004-000000000001', 'tag', 'infra'),
    ('0190a000-0000-7000-8004-000000000002', 'tag', 'storage');
