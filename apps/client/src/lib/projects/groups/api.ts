import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult } from '$lib/hub/hub';

/** A project group: the operator's own label, one for each project at most (PRO-059). */
export interface ProjectGroup {
  readonly id: string;
  readonly name: string;
  /** The projects of the group that are not left out (PRO-064). */
  readonly projectCount: number;
}

const GROUPS = graphql(`
  query ProjectGroups {
    projectGroups {
      id
      name
      projectCount
    }
  }
`);

const CREATE = graphql(`
  mutation CreateProjectGroup($name: String!, $projects: [ID!]!) {
    createProjectGroup(name: $name, projects: $projects) {
      id
      name
      projectCount
    }
  }
`);

const RENAME = graphql(`
  mutation RenameProjectGroup($id: ID!, $name: String!) {
    renameProjectGroup(id: $id, name: $name) {
      id
      name
      projectCount
    }
  }
`);

const DELETE = graphql(`
  mutation DeleteProjectGroup($id: ID!) {
    deleteProjectGroup(id: $id)
  }
`);

const SET = graphql(`
  mutation SetProjectsGroup($projects: [ID!]!, $group: ID) {
    setProjectsGroup(projects: $projects, group: $group)
  }
`);

/** The group calls of the hub (PRO-059 to PRO-063). */
export interface GroupsApi {
  groups(): Promise<HubResult<readonly ProjectGroup[]>>;
  /** Makes the group with these projects in it; a project in a different group moves (PRO-060). */
  createGroup(name: string, projects: readonly string[]): Promise<HubResult<ProjectGroup>>;
  renameGroup(id: string, name: string): Promise<HubResult<ProjectGroup>>;
  /** Removes the group only; its projects stay, with no group (PRO-062). */
  deleteGroup(id: string): Promise<HubResult<string>>;
  /** Puts the projects in the group, or in none with null (PRO-060, PRO-063). */
  setGroup(projects: readonly string[], group: string | null): Promise<HubResult<readonly string[]>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function groupsApi(hub: Pick<Hub, 'request'>): GroupsApi {
  return {
    groups: async () => mapValue(await hub.request(GROUPS, {}), (value) => value.projectGroups),
    createGroup: async (name, projects) => mapValue(await hub.request(CREATE, { name, projects: [...projects] }), (value) => value.createProjectGroup),
    renameGroup: async (id, name) => mapValue(await hub.request(RENAME, { id, name }), (value) => value.renameProjectGroup),
    deleteGroup: async (id) => mapValue(await hub.request(DELETE, { id }), (value) => value.deleteProjectGroup),
    setGroup: async (projects, group) => mapValue(await hub.request(SET, { projects: [...projects], group }), (value) => value.setProjectsGroup),
  };
}
