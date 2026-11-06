CREATE SCHEMA maintenance;

CREATE TABLE maintenance.windows (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    kind text NOT NULL,
    timezone text NOT NULL,
    rule jsonb NOT NULL,
    active_manual boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT windows_name_key UNIQUE (name),
    CONSTRAINT windows_kind_check CHECK (
        kind IN ('manual', 'single', 'recurring_interval', 'recurring_weekday', 'recurring_day_of_month')
    ),
    -- The rule carries its own kind tag; the column and the tag must agree.
    CONSTRAINT windows_rule_kind_check CHECK (rule ->> 'kind' = kind),
    -- Only a manual window has an on/off state.
    CONSTRAINT windows_active_manual_check CHECK (kind = 'manual' OR NOT active_manual)
);

CREATE TABLE maintenance.window_targets (
    window_id uuid NOT NULL,
    target_type text NOT NULL,
    target text NOT NULL,
    CONSTRAINT window_targets_pkey PRIMARY KEY (window_id, target_type, target),
    CONSTRAINT window_targets_window_id_fkey FOREIGN KEY (window_id)
        REFERENCES maintenance.windows (id) ON DELETE CASCADE,
    CONSTRAINT window_targets_target_type_check CHECK (target_type IN ('device', 'check', 'tag'))
);
