import { graphql } from '$lib/hub/generated';
import type {
  AcceptWebInterfaceSuggestionMutation,
  CreateWebInterfaceMutation,
  OpenIn,
  UpdateWebInterfaceMutation,
  WebInterfaceInput,
  WebInterfaceSuggestionFieldsFragment,
} from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// The device queries and the mutations below spread these fragments (K1 `Device.webInterfaces`, `Device.webInterfaceSuggestions`).
void graphql(`
  fragment WebInterfaceSuggestionFields on WebInterfaceSuggestion {
    checkId
    url
    label
  }
`);

void graphql(`
  fragment WebInterfaceFields on WebInterface {
    id
    deviceId
    label
    url
    openIn
    pinnedFingerprintSha256
    pinnedAt
  }
`);

const CreateWebInterface = graphql(`
  mutation CreateWebInterface($deviceId: ID!, $input: WebInterfaceInput!) {
    createWebInterface(deviceId: $deviceId, input: $input) {
      ...WebInterfaceFields
    }
  }
`);

const UpdateWebInterface = graphql(`
  mutation UpdateWebInterface($id: ID!, $input: WebInterfaceInput!) {
    updateWebInterface(id: $id, input: $input) {
      ...WebInterfaceFields
    }
  }
`);

const DeleteWebInterface = graphql(`
  mutation DeleteWebInterface($id: ID!) {
    deleteWebInterface(id: $id)
  }
`);

const AcceptWebInterfaceSuggestion = graphql(`
  mutation AcceptWebInterfaceSuggestion($checkId: ID!) {
    acceptWebInterfaceSuggestion(checkId: $checkId) {
      ...WebInterfaceFields
    }
  }
`);

const DismissWebInterfaceSuggestion = graphql(`
  mutation DismissWebInterfaceSuggestion($checkId: ID!) {
    dismissWebInterfaceSuggestion(checkId: $checkId)
  }
`);

/** A web interface of a device (WEB-001); the pin is set when its certificate is accepted (WEB-008). */
export type WebInterface = CreateWebInterfaceMutation['createWebInterface'];

/** A web interface the hub offers from a passing HTTP check (WEB-002). */
export type WebInterfaceSuggestion = WebInterfaceSuggestionFieldsFragment;

export type { OpenIn, WebInterfaceInput };

/** The web interface mutations of one device's settings (WEB-001, WEB-002) through a `Hub` (CQ-045). */
export function webInterfacesApi(hub: Hub) {
  return {
    create: async (deviceId: string, input: WebInterfaceInput): Promise<HubResult<WebInterface>> => {
      const result = await hub.request(CreateWebInterface, { deviceId, input });
      return result.ok ? { ok: true, value: result.value.createWebInterface } : result;
    },
    update: async (id: string, input: WebInterfaceInput): Promise<HubResult<UpdateWebInterfaceMutation['updateWebInterface']>> => {
      const result = await hub.request(UpdateWebInterface, { id, input });
      return result.ok ? { ok: true, value: result.value.updateWebInterface } : result;
    },
    /** Resolves to the deleted ID. */
    delete: async (id: string): Promise<HubResult<string>> => {
      const result = await hub.request(DeleteWebInterface, { id });
      return result.ok ? { ok: true, value: result.value.deleteWebInterface } : result;
    },
    /** Makes a web interface from the suggestion of a check. */
    acceptSuggestion: async (
      checkId: string,
    ): Promise<HubResult<AcceptWebInterfaceSuggestionMutation['acceptWebInterfaceSuggestion']>> => {
      const result = await hub.request(AcceptWebInterfaceSuggestion, { checkId });
      return result.ok ? { ok: true, value: result.value.acceptWebInterfaceSuggestion } : result;
    },
    /** Resolves to the dismissed check's ID. */
    dismissSuggestion: async (checkId: string): Promise<HubResult<string>> => {
      const result = await hub.request(DismissWebInterfaceSuggestion, { checkId });
      return result.ok ? { ok: true, value: result.value.dismissWebInterfaceSuggestion } : result;
    },
  };
}
