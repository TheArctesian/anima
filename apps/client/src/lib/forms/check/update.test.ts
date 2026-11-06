import { describe, expect, it } from 'vitest';
import { saved } from './check.fixture';
import { updateCheckInput } from './model/input';
import { checkValues } from './model/values';

describe('update keeps the type of the saved check', () => {
  const tcp = saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 });

  it('CHK-009: type fixed after create: a config change is built for the saved type even if the values say another', () => {
    const initial = checkValues(tcp, 'ICMP');
    const edited = checkValues(tcp, 'ICMP');
    edited.tcp.port = '2222';
    edited.type = 'HTTP';

    expect(updateCheckInput(initial, edited)).toEqual({ config: { tcp: { port: 2222, timeoutMs: 3000 } } });
  });

  it('CHK-009: type fixed after create: a changed type alone changes nothing', () => {
    const initial = checkValues(tcp, 'ICMP');
    const edited = checkValues(tcp, 'ICMP');
    edited.type = 'DNS';
    edited.dns.name = 'example.com';

    expect(updateCheckInput(initial, edited)).toEqual({});
  });
});
