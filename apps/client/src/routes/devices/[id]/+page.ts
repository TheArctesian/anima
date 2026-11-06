import { parseDeviceId } from './device-id';
import type { PageLoad } from './$types';

/** The ID is parsed at the route edge (D-14); a malformed one is null and the page says "Device not found". */
export const load: PageLoad = ({ params }) => ({ id: parseDeviceId(params.id) });
