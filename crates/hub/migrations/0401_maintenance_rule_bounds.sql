-- A stored rule keeps the bounds the form enforces (MW-004, API-005). Evaluating a rule costs
-- a step per day of its duration, so a huge stored duration would slow every check attempt.
-- Each field is bounded only when present, and must then be the right JSON type; the CASE
-- keeps a cast from running on the wrong type.
ALTER TABLE maintenance.windows ADD CONSTRAINT windows_rule_bounds_check CHECK (
    (rule -> 'durationMinutes' IS NULL
        OR (CASE WHEN jsonb_typeof(rule -> 'durationMinutes') = 'number'
                 THEN (rule ->> 'durationMinutes')::numeric BETWEEN 1 AND 10080
                 ELSE false END))
    AND (rule -> 'everyDays' IS NULL
        OR (CASE WHEN jsonb_typeof(rule -> 'everyDays') = 'number'
                 THEN (rule ->> 'everyDays')::numeric BETWEEN 1 AND 2147483647
                 ELSE false END))
    AND (rule -> 'weekdays' IS NULL
        OR (CASE WHEN jsonb_typeof(rule -> 'weekdays') = 'array'
                 THEN jsonb_array_length(rule -> 'weekdays') BETWEEN 1 AND 7
                 ELSE false END))
    AND (rule -> 'daysOfMonth' IS NULL
        OR (CASE WHEN jsonb_typeof(rule -> 'daysOfMonth') = 'array'
                 THEN jsonb_array_length(rule -> 'daysOfMonth') BETWEEN 1 AND 32
                 ELSE false END))
);
