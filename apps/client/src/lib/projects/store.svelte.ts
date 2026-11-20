import { SvelteSet } from 'svelte/reactivity';
import type { HubResult } from '$lib/hub/hub';
import type { Project, ProjectBrowse, ProjectResource, ProjectsApi } from './api';
import type { ProjectGroup } from './groups/api';
import { withGroup, withGroupNamed } from './groups/view';
import { isLeftOut } from './words';

export type ProjectsStatus = 'loading' | 'ready' | 'failed';

/** The hub's words for an action it refused. */
export interface Refusal {
  readonly message: string;
}

type View = 'main' | 'leftOut';

/**
 * The projects the hub lists (PRO-001). A change shows at once, laid over what the hub last said; a refused change
 * drops out, the entry goes back, and `refusal` holds the hub's words.
 */
export class ProjectsStore {
  status = $state<ProjectsStatus>('loading');
  loadProblem = $state('');
  refusal = $state.raw<Refusal | null>(null);
  /** The one project the detail page asked for by slug: undefined while unread, null when the hub has none. */
  found = $state.raw<Project | null | undefined>(undefined);
  #generation = 0;
  #browse = $state.raw<ProjectBrowse>({});
  #known = $state.raw<readonly Project[]>([]);
  #next = $state.raw<Readonly<Record<View, string | null>>>({ main: null, leftOut: null });
  /** Repository IDs mid-change, with the leftOut they are heading to. */
  #leaving = $state.raw<Readonly<Record<string, boolean>>>({});
  /** Resource IDs mid-confirm. */
  #confirming = $state.raw<readonly string[]>([]);

  languages = $state.raw<readonly string[]>([]);
  /** The operator's project groups (PRO-059). */
  groups = $state.raw<readonly ProjectGroup[]>([]);
  readonly browse = $derived(this.#browse);
  readonly byName = $derived(this.#browse.order == null || this.#browse.order === 'NAME');
  readonly filtered = $derived(
    this.#browse.language != null || this.#browse.minSizeKib != null || this.#browse.maxSizeKib != null || this.#browse.group != null || this.#browse.ungrouped === true,
  );

  /** Keep the hub's order for metadata sorts, with local changes laid over it. */
  readonly shown = $derived.by(() => {
    const projects = this.#known.map((project) => this.#overlaid(project));
    return this.byName ? projects.toSorted((a, b) => a.name.localeCompare(b.name)) : projects;
  });
  readonly main = $derived(this.shown.filter((project) => !isLeftOut(project)));
  readonly leftOut = $derived(this.shown.filter((project) => isLeftOut(project)));
  /** The detail page's project, with the changes laid over it. */
  readonly current = $derived(this.found == null ? this.found : this.#overlaid(this.#latest(this.found)));

  constructor(private readonly api: ProjectsApi) {}

  hasMore(view: View): boolean {
    return this.#next[view] !== null;
  }

  async setBrowse(change: ProjectBrowse): Promise<void> {
    this.#browse = { ...this.#browse, ...change };
    await this.load();
  }

  async load(): Promise<void> {
    const generation = ++this.#generation;
    const browse = this.#browse;
    this.status = 'loading';
    const [main, left, groups] = await Promise.all([this.api.list(false, null, browse), this.api.list(true, null, browse), this.api.groups()]);
    if (generation !== this.#generation) return;
    if (!main.ok || !left.ok || !groups.ok) {
      this.loadProblem = [main, left, groups].map((each) => (each.ok ? '' : each.error.message)).find((message) => message !== '') ?? '';
      this.status = 'failed';
      return;
    }
    this.groups = groups.value;
    this.#known = [];
    this.#merge(main.value.projects);
    this.#merge(left.value.projects);
    this.#next = { main: main.value.next, leftOut: left.value.next };
    this.languages = [...new SvelteSet([...(main.value.languages ?? []), ...(left.value.languages ?? [])])].sort((a, b) => a.localeCompare(b));
    this.loadProblem = '';
    this.status = 'ready';
  }

  /** Reads the next page of one view. */
  async more(view: View): Promise<void> {
    const after = this.#next[view];
    if (after === null) return;
    const generation = this.#generation;
    const result = await this.api.list(view === 'leftOut', after, this.#browse);
    if (generation !== this.#generation || after !== this.#next[view]) return;
    if (!result.ok) {
      this.refusal = { message: result.error.message };
      return;
    }
    this.#merge(result.value.projects);
    this.#next = { ...this.#next, [view]: result.value.next };
  }

  /** Reads one project for the detail page. */
  async open(slug: string): Promise<void> {
    this.status = 'loading';
    const result = await this.api.find(slug);
    if (!result.ok) {
      this.loadProblem = result.error.message;
      this.status = 'failed';
      return;
    }
    this.found = result.value;
    if (result.value !== null) this.#merge([result.value]);
    this.loadProblem = '';
    this.status = 'ready';
  }

  /** Leaves out a repository (PRO-002); the entry goes at once. */
  leaveOut(repositoryId: string): Promise<void> {
    return this.#setLeftOut(repositoryId, true);
  }

  restore(repositoryId: string): Promise<void> {
    return this.#setLeftOut(repositoryId, false);
  }

  /** Confirms a proposed resource (PRO-004); it shows as confirmed at once. */
  async confirm(resourceId: string): Promise<void> {
    this.refusal = null;
    this.#confirming = [...this.#confirming, resourceId];
    const result = await this.api.confirm(resourceId);
    if (result.ok) this.#replaceResource(result.value);
    else this.refusal = { message: result.error.message };
    this.#confirming = this.#confirming.filter((each) => each !== resourceId);
  }

  /** Makes a group with the projects in it (PRO-060); the hub's words come back when it refuses. */
  async createGroup(name: string, projectIds: readonly string[]): Promise<HubResult<unknown>> {
    const result = await this.api.createGroup(name, projectIds);
    if (result.ok) await this.#grouped(projectIds, { id: result.value.id, name: result.value.name });
    return result;
  }

  /** Puts the projects in the group, or in none with null (PRO-060, PRO-063). */
  async setGroup(projectIds: readonly string[], groupId: string | null): Promise<void> {
    this.refusal = null;
    const result = await this.api.setGroup(projectIds, groupId);
    if (!result.ok) {
      this.refusal = { message: result.error.message };
      return;
    }
    const group = this.groups.find((each) => each.id === groupId);
    await this.#grouped(projectIds, group === undefined ? null : { id: group.id, name: group.name });
  }

  async renameGroup(id: string, name: string): Promise<HubResult<unknown>> {
    const result = await this.api.renameGroup(id, name);
    if (result.ok) {
      this.groups = this.groups.map((each) => (each.id === id ? result.value : each));
      this.#known = withGroupNamed(this.#known, id, result.value.name);
    }
    return result;
  }

  /** Deletes the group only: its projects stay, with no group (PRO-062). */
  async deleteGroup(id: string): Promise<HubResult<unknown>> {
    const result = await this.api.deleteGroup(id);
    if (!result.ok) return result;
    this.groups = this.groups.filter((each) => each.id !== id);
    this.#known = withGroupNamed(this.#known, id, null);
    if (this.#browse.group === id) await this.setBrowse({ group: null });
    return result;
  }

  /** Lays the new group over the projects, then reads the counts again; a group filter reads the list again. */
  async #grouped(projectIds: readonly string[], group: Project['group']): Promise<void> {
    this.#known = withGroup(this.#known, projectIds, group);
    if (this.#browse.group != null || this.#browse.ungrouped === true) {
      await this.load();
      return;
    }
    const groups = await this.api.groups();
    if (groups.ok) this.groups = groups.value;
  }

  async #setLeftOut(repositoryId: string, leftOut: boolean): Promise<void> {
    this.refusal = null;
    this.#leaving = { ...this.#leaving, [repositoryId]: leftOut };
    const result = await (leftOut ? this.api.leaveOut(repositoryId) : this.api.restore(repositoryId));
    if (result.ok) {
      this.#merge([result.value]);
      if (this.filtered || !this.byName) await this.load();
    } else this.refusal = { message: result.error.message };
    this.#leaving = Object.fromEntries(Object.entries(this.#leaving).filter(([id]) => id !== repositoryId));
  }

  #latest(project: Project): Project {
    return this.#known.find((each) => each.id === project.id) ?? project;
  }

  #merge(projects: readonly Project[]): void {
    const ids = projects.map((project) => project.id);
    this.#known = [...this.#known.filter((each) => !ids.includes(each.id)), ...projects];
  }

  #replaceResource(resource: ProjectResource): void {
    this.#known = this.#known.map((project) => ({
      ...project,
      resources: project.resources.map((each) => (each.id === resource.id ? resource : each)),
    }));
  }

  #overlaid(project: Project): Project {
    return {
      ...project,
      repositories: project.repositories.map((repository) => ({ ...repository, leftOut: this.#leaving[repository.id] ?? repository.leftOut })),
      resources: project.resources.map((resource) => (this.#confirming.includes(resource.id) ? { ...resource, state: 'CONFIRMED' } : resource)),
    };
  }
}
