import Guacamole from './vendor/guacamole.js';
import type { SessionTunnel } from './guacamole/tunnel';

/** The part of the Guacamole client an argument value is written to. */
export type ArgumentClient = Pick<SessionTunnel['client'], 'createArgumentValueStream'>;

const PLAIN_TEXT = 'text/plain';
const UTF_8 = new TextEncoder();

/**
 * Ask the gateway to set connection parameter `name` to `value`: one `text/plain` argument value stream, written
 * and closed (GW-015). The text goes as UTF-8 bytes through `ArrayBufferWriter`, because the 1.5.5 `StringWriter`
 * encodes each UTF-16 unit on its own and would mangle a character outside the BMP. An empty value is a stream
 * with no blob, as `StringWriter` sends it.
 */
export function sendArgument(client: ArgumentClient, name: string, value: string): void {
  const writer = new Guacamole.ArrayBufferWriter(client.createArgumentValueStream(PLAIN_TEXT, name));
  const bytes = UTF_8.encode(value);
  if (bytes.length > 0) writer.sendData(bytes);
  writer.sendEnd();
}
