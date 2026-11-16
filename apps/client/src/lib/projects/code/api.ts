import { graphql } from '$lib/hub/generated';
import { field, type Asker } from '../forge/request';

const CODE_DIRECTORY = graphql(`
  query ForgeCodeDirectory($repository: ID!, $ref: String, $path: String, $fresh: Boolean) {
    codeDirectory(repository: $repository, ref: $ref, path: $path, fresh: $fresh) {
      ref
      path
      entries {
        name
        path
        kind
        size
      }
      partial
      read {
        ...ForgeReadFields
      }
    }
  }
`);

const CODE_FILE = graphql(`
  query ForgeCodeFile($repository: ID!, $ref: String, $path: String!) {
    codeFile(repository: $repository, ref: $ref, path: $path) {
      path
      name
      size
      content
      text
      url
    }
  }
`);

/** The hub's calls for the Code tab: a directory is open to a read key, a file's bytes only to a signed-in operator (PRJ-035). */
export const codeApi = (hub: Asker) => ({
  directory: (repository: string, ref: string | null, path: string, fresh = false) =>
    field(hub, CODE_DIRECTORY, 'codeDirectory', { repository, ref, path, fresh }),
  file: (repository: string, ref: string | null, path: string) => field(hub, CODE_FILE, 'codeFile', { repository, ref, path }),
});
