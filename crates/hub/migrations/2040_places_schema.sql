-- The places domain's two tables (K-E8 range 2040-2049; PLC-001 to PLC-006).
--
-- A top-level place has a centre and a radius; a child place has none (PLC-002). The code
-- checks which places are top-level and how deep the tree is; the database keeps the ranges.
-- `site_id` is a `SiteId` of `devices`, held as a value: no foreign key leaves this schema.
-- Privileges: migration 0001's default privileges give `anima_runtime` what it needs on the
-- new schema (K2 "Roles").
create schema places;

create table places.place (
    id            uuid             primary key,
    parent_id     uuid             references places.place (id),
    name          text             not null check (length(name) between 1 and 120),
    latitude      double precision check (latitude between -90 and 90),
    longitude     double precision check (longitude between -180 and 180),
    radius_m      integer          check (radius_m between 1 and 100000),
    category      text             not null check (category in
                      ('home', 'work', 'site', 'shop', 'leisure', 'travel', 'storage', 'other')),
    icon          text             not null check (icon ~ '^[a-z][a-z0-9-]{0,39}$'),
    aliases       text[]           not null default '{}' check (cardinality(aliases) <= 20),
    geocoder_name text             check (length(geocoder_name) between 1 and 120),
    merged_into   uuid             references places.place (id),
    created_at    timestamptz      not null,
    updated_at    timestamptz      not null,
    check ((latitude is null) = (longitude is null)),
    check (parent_id is null or (latitude is null and longitude is null and radius_m is null)),
    check (parent_id is distinct from id),
    check (merged_into is distinct from id)
);

create index place_parent on places.place (parent_id);
create index place_merged_into on places.place (merged_into);

create table places.site_place (
    site_id  uuid primary key,
    place_id uuid not null references places.place (id)
);

create index site_place_place on places.site_place (place_id);
