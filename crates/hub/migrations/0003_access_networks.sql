-- The client networks the hub serves (K2, NET-001, NET-002, D-10).
-- `access::permit_networks` keeps the rows labelled `ANIMA_PERMITTED_NETWORKS` equal to that
-- variable at every start; rows with another label are never touched by it.

create table access.permitted_networks (
    id uuid primary key,
    -- `cidr` refuses host bits, so `10.0.0.1/8` cannot be stored for `10.0.0.0/8`.
    cidr cidr not null unique,
    label text not null check (char_length(label) between 1 and 128)
);

create index permitted_networks_label on access.permitted_networks (label);

-- No grants here: anima_runtime gets its table privileges from the default privileges that
-- migration 0001 sets for the owner role (R-016, K2 "Roles").
