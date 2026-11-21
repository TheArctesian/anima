-- The model library of the compute domain: the models, their variants and files, the aliases
-- and the approved sources (K2 1040-1059, ruling R-NEW-modellibrary; LIBM-001 to
-- LIBM-008, SEL-003).
--
-- `approved_sources` holds the Hugging Face organisations from which a variant can come
-- (LIBM-003); `unsloth` and `mlx-community` are there from the start, and the operator can add
-- one. The model publisher is kept on its model (`models.publisher`). `models` holds the facts
-- that the hub read from the model files (LIBM-005); a null is a fact the files did not give.
-- `variants` holds each variant of a model by its tag (`name:tag`, LIBM-006), with the
-- repository and the commit its file list is from; `variant_files` holds each file with its
-- size and its SHA-256 value, which a pull verifies (PULL-005, SEC-C-009). `aliases` points a
-- name such as `coder` at one model; the hub refuses an alias that is the name of a model.
--
-- `library_settings` holds one row: how the publisher of a base model gets onto the approved
-- list (`base_model`, the operator's choice, or `by_hand`; R-NEW-modellibrary).
--
-- Deleting a model deletes its variants, their files and its aliases.
--
-- Privileges (K2 "Roles"): migration 0001's default privileges give `anima_runtime` USAGE on
-- the new schema and SELECT, INSERT, UPDATE and DELETE on the new tables. The migration only
-- creates small tables, so it is cheap on a large database (DB-010).

create schema compute;

create table compute.approved_sources (
    organisation text not null,
    added_at timestamptz not null,
    constraint approved_sources_pkey primary key (organisation),
    constraint approved_sources_organisation_check
        check (organisation ~ '^[A-Za-z0-9_-][A-Za-z0-9._-]{0,95}$')
);

-- Hugging Face compares organisation names without case.
create unique index approved_sources_lower_idx on compute.approved_sources (lower(organisation));

insert into compute.approved_sources (organisation, added_at)
values ('unsloth', now()), ('mlx-community', now());

create table compute.library_settings (
    only_row boolean not null default true,
    publisher_approval text not null,
    constraint library_settings_pkey primary key (only_row),
    constraint library_settings_only_row_check check (only_row),
    constraint library_settings_publisher_approval_check
        check (publisher_approval in ('base_model', 'by_hand'))
);

insert into compute.library_settings (publisher_approval) values ('base_model');

create table compute.models (
    id uuid not null,
    name text not null,
    family text null,
    parameter_count bigint null,
    context_length integer null,
    layers integer null,
    hidden_size integer null,
    attention_heads integer null,
    key_value_heads integer null,
    capabilities text[] not null default '{}',
    licence text null,
    publisher text null,
    added_at timestamptz not null,
    constraint models_pkey primary key (id),
    constraint models_name_key unique (name),
    constraint models_name_check check (name ~ '^[a-z0-9][a-z0-9._-]{0,63}$'),
    constraint models_family_check check (char_length(family) between 1 and 64),
    constraint models_parameter_count_check check (parameter_count > 0),
    constraint models_context_length_check check (context_length > 0),
    constraint models_shape_check check (
        layers > 0 and hidden_size > 0 and attention_heads > 0 and key_value_heads > 0
    ),
    -- Fixed identifiers of LIBM-008: never change the meaning of one; add a new one instead.
    constraint models_capabilities_check check (capabilities <@ array[
        'chat', 'tools', 'coding', 'reasoning', 'translation', 'summarization', 'text_classification',
        'structured_extraction', 'token_classification', 'image_generation', 'image_editing',
        'upscaling', 'image_classification', 'video_generation', 'video_editing',
        'video_understanding', 'video_classification', 'music_generation', 'audio_generation',
        'audio_understanding', 'audio_classification', 'audio_to_audio', 'transcription',
        'spoken_translation', 'text_to_speech', 'conversational_voice', 'image_understanding',
        'ocr', 'document_field_extraction', 'document_question_answering',
        'visual_document_retrieval', 'object_detection', 'image_segmentation',
        'depth_estimation', 'pose_detection', 'embeddings', 'reranking',
        'tabular_classification', 'tabular_regression', 'policy',
        'three_dimensional_generation'
    ]::text[]),
    constraint models_licence_check check (char_length(licence) between 1 and 64),
    constraint models_publisher_check
        check (publisher ~ '^[A-Za-z0-9_-][A-Za-z0-9._-]{0,95}$')
);

create table compute.variants (
    model_id uuid not null,
    tag text not null,
    format text not null,
    quantisation text not null,
    repository text not null,
    revision text not null,
    constraint variants_pkey primary key (model_id, tag),
    constraint variants_model_id_fkey foreign key (model_id)
        references compute.models (id) on delete cascade,
    constraint variants_tag_check check (tag ~ '^[a-z0-9][a-z0-9._-]{0,63}$'),
    constraint variants_format_check check (format in ('gguf', 'mlx', 'safetensors')),
    constraint variants_quantisation_check check (char_length(quantisation) between 1 and 64),
    constraint variants_repository_check
        check (repository ~ '^[A-Za-z0-9_-][A-Za-z0-9._-]{0,95}/[A-Za-z0-9_-][A-Za-z0-9._-]{0,95}$'),
    constraint variants_revision_check check (revision ~ '^[0-9a-f]{40}$')
);

create table compute.variant_files (
    model_id uuid not null,
    tag text not null,
    path text not null,
    size_bytes bigint not null,
    sha256 text not null,
    constraint variant_files_pkey primary key (model_id, tag, path),
    constraint variant_files_variant_fkey foreign key (model_id, tag)
        references compute.variants (model_id, tag) on delete cascade,
    constraint variant_files_path_check check (octet_length(path) between 1 and 1024),
    constraint variant_files_size_bytes_check check (size_bytes >= 0),
    constraint variant_files_sha256_check check (sha256 ~ '^[0-9a-f]{64}$')
);

create table compute.aliases (
    alias text not null,
    model_id uuid not null,
    constraint aliases_pkey primary key (alias),
    constraint aliases_model_id_fkey foreign key (model_id)
        references compute.models (id) on delete cascade,
    constraint aliases_alias_check check (alias ~ '^[a-z0-9][a-z0-9._-]{0,63}$')
);

create index aliases_model_id_idx on compute.aliases (model_id);
