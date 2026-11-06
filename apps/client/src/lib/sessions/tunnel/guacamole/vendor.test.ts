// @vitest-environment jsdom
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import Guacamole from '../vendor/guacamole.js';

/**
 * SHA-256 of `vendor/guacamole.js` without its last line (`export default Guacamole;`), which is the Apache
 * Guacamole 1.5.5 modules joined in name order (R-045, R-046). Re-derive it from the signed release:
 *   tar -xzf guacamole-client-1.5.5.tar.gz guacamole-client-1.5.5/guacamole-common-js/src/main/webapp/modules
 *   cd guacamole-client-1.5.5/guacamole-common-js/src/main/webapp/modules
 *   LC_ALL=C cat $(ls *.js | LC_ALL=C sort) | shasum -a 256
 */
const APACHE_1_5_5_MODULES_SHA256 = 'd5c167c0c3c40aae5fa25177dc56465b947ec24ab1a5a7f60e7009684525c9df';
const EXPORT_LINE = 'export default Guacamole;\n';

const vendored = readFileSync(join(import.meta.dirname, '..', 'vendor', 'guacamole.js'), 'utf8');

describe('vendored Guacamole client (R-045)', () => {
  it('R-046: is the Apache 1.5.5 modules byte for byte, plus one export line', () => {
    expect(vendored.endsWith(EXPORT_LINE)).toBe(true);
    const modules = vendored.slice(0, -EXPORT_LINE.length);
    expect(createHash('sha256').update(modules, 'utf8').digest('hex')).toBe(APACHE_1_5_5_MODULES_SHA256);
  });

  it('R-045: keeps the Apache licence header of every module', () => {
    expect(vendored.match(/Licensed to the Apache Software Foundation \(ASF\)/g)).toHaveLength(34);
  });

  it('R-046: exports the classes the tunnel uses; the 1.5.5 source still reports API version 1.5.4', () => {
    expect((Guacamole as unknown as { API_VERSION: string }).API_VERSION).toBe('1.5.4');
    for (const name of ['Client', 'WebSocketTunnel', 'Status', 'StringWriter', 'StringReader', 'Keyboard', 'Mouse'] as const) {
      expect(typeof Guacamole[name]).toBe('function');
    }
  });
});
