-- Project groups (PRO-059 to PRO-064). A group is the operator's own label: a sync never makes,
-- changes or removes one. A project is in at most one group (`projects.group_id`), and a group
-- holds no group. To delete a group sets the group of its projects to null: it never deletes a
-- project and never leaves one out (PRO-062). Two names that differ only in the case of the
-- letters are one name.
create table projects.groups (
    id         uuid        primary key,
    name       text        not null check (length(name) between 1 and 60),
    created_at timestamptz not null
);

create unique index groups_name on projects.groups (lower(name));

alter table projects.projects
    add column group_id uuid null references projects.groups (id) on delete set null;

create index projects_group on projects.projects (group_id);
