import { describe, expect, it } from 'vitest';
import { DEFAULT_QUERY, listVariables, parseListQuery, writeListQuery, type ListQuery } from './query';

describe('list query in the URL (D-4)', () => {
  it('DEV-003, DEV-004 reads the defaults from an empty URL', () => {
    expect(parseListQuery(new URLSearchParams())).toEqual(DEFAULT_QUERY);
  });

  it('DEV-003, DEV-004 round-trips every part and writes them in a fixed order', () => {
    const query: ListQuery = {
      view: 'table',
      name: 'nas',
      statuses: ['UP', 'DOWN'],
      tags: ['lab', 'home'],
      sort: 'UPTIME',
    };
    const written = writeListQuery(query).toString();
    expect(written).toBe('view=table&q=nas&status=UP&status=DOWN&tag=lab&tag=home&sort=UPTIME');
    expect(parseListQuery(new URLSearchParams(written))).toEqual(query);
  });

  it('DEV-003, DEV-004 writes nothing for the defaults', () => {
    expect(writeListQuery(DEFAULT_QUERY).toString()).toBe('');
  });

  it('DEV-003, DEV-004 drops unknown values instead of failing', () => {
    const params = new URLSearchParams('view=cards&status=BROKEN&status=DOWN&sort=SIZE&tag=&tag=lab&tag=lab');
    expect(parseListQuery(params)).toEqual({ ...DEFAULT_QUERY, statuses: ['DOWN'], tags: ['lab'] });
  });
});

describe('list variables (DEV-001, DEV-003, DEV-004)', () => {
  it('DEV-001 asks for 200 devices and omits an empty filter', () => {
    expect(listVariables(DEFAULT_QUERY)).toStrictEqual({ sort: 'NAME', first: 200 });
  });

  it('DEV-003 sends only the filter parts that are set', () => {
    expect(listVariables({ ...DEFAULT_QUERY, name: 'nas' }).filter).toEqual({ name: 'nas' });
    expect(listVariables({ ...DEFAULT_QUERY, statuses: ['DOWN'], tags: ['lab'] }).filter).toEqual({
      status: ['DOWN'],
      tags: ['lab'],
    });
  });

  it('DEV-004 passes the sort through and continues from a cursor', () => {
    expect(listVariables({ ...DEFAULT_QUERY, sort: 'STATUS' }, 'cursor-9')).toStrictEqual({
      sort: 'STATUS',
      first: 200,
      after: 'cursor-9',
    });
  });
});
