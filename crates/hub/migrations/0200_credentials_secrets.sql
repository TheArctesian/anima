CREATE SCHEMA credentials;

-- Sealed device secrets (SEC-003): only nonce, ciphertext and key id are stored, never plaintext.
-- created_at defaults to now(): store_ssh takes no clock (K3).
CREATE TABLE credentials.secrets (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL,
    kind text NOT NULL,
    nonce bytea NOT NULL,
    ciphertext bytea NOT NULL,
    key_id text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT secrets_device_id_fkey FOREIGN KEY (device_id)
        REFERENCES devices.devices (id) ON DELETE CASCADE,
    CONSTRAINT secrets_device_id_kind_key UNIQUE (device_id, kind),
    CONSTRAINT secrets_kind_check
        CHECK (kind IN ('ssh_password', 'ssh_private_key', 'ssh_passphrase'))
);
