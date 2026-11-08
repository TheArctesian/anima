import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
  buildSchema,
  defaultFieldResolver,
  getNamedType,
  isEnumType,
  isListType,
  isNonNullType,
  isObjectType,
  isScalarType,
  type GraphQLFieldResolver,
  type GraphQLOutputType,
  type GraphQLSchema,
} from 'graphql';

// The real K1 schema, with its `@pending` directive declared in the file, so it builds as it is.
const SCHEMA_FILE = fileURLToPath(new URL('../../../../contracts/schema.graphql', import.meta.url));

export function loadSchema(): GraphQLSchema {
  return buildSchema(readFileSync(SCHEMA_FILE, 'utf8'));
}

function scalarDefault(name: string): unknown {
  switch (name) {
    case 'Boolean':
      return false;
    case 'Int':
    case 'Float':
      return 0;
    case 'DateTime':
      return '2026-11-01T00:00:00Z';
    default:
      return '';
  }
}

/** What a field the fixtures do not hold answers: null when it may be, else the emptiest value of its type. */
function emptyValue(type: GraphQLOutputType): unknown {
  if (!isNonNullType(type)) return null;
  const inner = type.ofType;
  if (isListType(inner)) return [];
  const named = getNamedType(inner);
  if (isEnumType(named)) return named.getValues()[0]?.value ?? null;
  if (isScalarType(named)) return scalarDefault(named.name);
  return isObjectType(named) ? {} : null;
}

/**
 * Reads the field from the source object; a function is called with the field's arguments. A field the source does not
 * hold answers `emptyValue`, so a query never fails on a gap in the fixtures.
 */
export const fieldFromFixtures: GraphQLFieldResolver<unknown, unknown> = (source, args, context, info) => {
  const held = defaultFieldResolver(source, args, context, info) as unknown;
  return held === undefined ? emptyValue(info.returnType) : held;
};
