// @vitest-environment jsdom
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import Guacamole from './vendor/guacamole.js';

/**
 * SHA-256 of `vendor/guacamole.js` without its last line (`export default Guacamole;`), which is the Apache
 * Guacamole 1.5.0 modules joined in name order (R-045). Re-derive it from the signed release:
 *   tar -xzf guacamole-client-1.5.0.tar.gz guacamole-client-1.5.0/guacamole-common-js/src/main/webapp/modules
 *   cd guacamole-client-1.5.0/guacamole-common-js/src/main/webapp/modules
 *   LC_ALL=C cat $(ls *.js | LC_ALL=C sort) | shasum -a 256
 */
const APACHE_1_5_0_MODULES_SHA256 = 'c2f557292789524c9093f6ba7e0769c5555c16a15b5a4546663576ac5efce147';
const EXPORT_LINE = 'export default Guacamole;\n';

const vendored = readFileSync(join(import.meta.dirname, 'vendor', 'guacamole.js'), 'utf8');

describe('vendored Guacamole client (R-045)', () => {
  it('R-045: is the Apache 1.5.0 modules byte for byte, plus one export line', () => {
    expect(vendored.endsWith(EXPORT_LINE)).toBe(true);
    const modules = vendored.slice(0, -EXPORT_LINE.length);
    expect(createHash('sha256').update(modules, 'utf8').digest('hex')).toBe(APACHE_1_5_0_MODULES_SHA256);
  });

  it('R-045: keeps the Apache licence header of every module', () => {
    expect(vendored.match(/Licensed to the Apache Software Foundation \(ASF\)/g)).toHaveLength(34);
  });

  it('R-045: reports API version 1.5.0 and exports the classes the tunnel uses', () => {
    expect((Guacamole as unknown as { API_VERSION: string }).API_VERSION).toBe('1.5.0');
    for (const name of ['Client', 'WebSocketTunnel', 'Status', 'StringWriter', 'StringReader', 'Keyboard', 'Mouse'] as const) {
      expect(typeof Guacamole[name]).toBe('function');
    }
  });
});
