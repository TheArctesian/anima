import { describe, expect, it } from 'vitest';
import { allowQuestion, consequencesOf } from './allowance';

describe('allowQuestion', () => {
  it('MCP-020: for everything, the title says so and the lines cover an API key and the own sign-in', () => {
    const question = allowQuestion({ type: 'bulk', state: 'allow' }, ['devices', 'checks', 'sessions', 'web', 'releases', 'retention']);
    expect(question.title).toBe('Allow every tool on this hub?');
    expect(question.body).toContain('the read tools too');
    expect(question.lines).toHaveLength(3);
    expect(question.lines[0]).toContain('end live sessions');
    expect(question.lines[0]).toContain('repoint web interfaces');
    expect(question.lines[1]).toContain('delete devices with their stored credentials');
    expect(question.lines[1]).toContain('start a release');
    expect(question.lines[1]).toContain('cut every data retention period');
  });

  it('MCP-020: for one set, only that set\'s consequences are listed', () => {
    const checks = allowQuestion({ type: 'bulk', toolset: 'checks', state: 'allow' }, ['devices', 'checks']);
    expect(checks.title).toBe('Allow every Checks tool?');
    expect(checks.lines.join(' ')).toContain('pause or delete checks');
    expect(checks.lines.join(' ')).not.toContain('delete devices');
    expect(checks.lines.join(' ')).not.toContain('sign-in');
    const releases = allowQuestion({ type: 'bulk', toolset: 'releases', state: 'allow' }, ['releases']);
    expect(releases.lines.join(' ')).toContain('start a release');
    expect(releases.lines.join(' ')).not.toContain('With an API key');
  });

  it('MCP-020: a set with no change tool lists nothing and says so', () => {
    const events = allowQuestion({ type: 'bulk', toolset: 'events', state: 'allow' }, ['events']);
    expect(events.lines).toEqual([]);
    expect(events.body).toContain('no tool that changes the hub');
  });

  it('MCP-020: sets the data does not know add no consequences', () => {
    expect(consequencesOf(['printers'])).toEqual({ key: [], signIn: [] });
  });
});
