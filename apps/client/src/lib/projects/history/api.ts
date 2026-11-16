import { graphql } from '$lib/hub/generated';
import { asked, field, paged, type Asker, type PageAsk } from '../forge/request';

// The fragment is only read by codegen, which inlines it into the document that spreads it (API-008).
void graphql(`
  fragment CommitFields on Commit {
    sha
    title
    authorName
    authorLogin
    committedAt
    url
    checks {
      ...CommitChecksFields
    }
  }
`);

const COMMITS = graphql(`
  query ForgeCommits($repository: ID!, $ref: String, $after: String, $fresh: Boolean) {
    commits(repository: $repository, ref: $ref, first: 30, after: $after, fresh: $fresh) {
      edges {
        node {
          ...CommitFields
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const BRANCHES = graphql(`
  query ForgeBranches($repository: ID!, $after: String, $fresh: Boolean) {
    branches(repository: $repository, first: 100, after: $after, fresh: $fresh) {
      edges {
        node {
          name
          sha
          protected
          default
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const RELEASES = graphql(`
  query ForgeReleases($repository: ID!, $after: String, $fresh: Boolean) {
    repositoryReleases(repository: $repository, first: 20, after: $after, fresh: $fresh) {
      edges {
        node {
          id
          tag
          name
          draft
          prerelease
          publishedAt
          createdAt
          url
        }
      }
      pageInfo {
        ...ForgePageFields
      }
      read {
        ...ForgeReadFields
      }
    }
  }
`);

/** The hub's calls for the History tab, all open to a read key (PRJ-031, PRJ-032). */
export const historyApi = (hub: Asker) => ({
  commits: async (repository: string, ref: string | null, ask: PageAsk = {}) =>
    paged(await field(hub, COMMITS, 'commits', { ...asked(repository, ask), ref })),
  branches: async (repository: string, ask: PageAsk = {}) => paged(await field(hub, BRANCHES, 'branches', asked(repository, ask))),
  releases: async (repository: string, ask: PageAsk = {}) => paged(await field(hub, RELEASES, 'repositoryReleases', asked(repository, ask))),
});
