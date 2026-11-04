/**
 * The K1 shapes the device form reads and writes. The codegen emits no input types without
 * documents (E-1 pending), so these match `contracts/schema.graphql` exactly.
 */
export type OperatingSystem = 'LINUX' | 'MACOS' | 'WINDOWS' | 'OTHER';
export type SshAuth = 'PASSWORD' | 'PRIVATE_KEY';

export interface HostKey {
  readonly algorithm: string;
  readonly fingerprintSha256: string;
  readonly firstSeenAt: string;
}

/** The part of K1 `Device` the form edits. */
export interface DeviceFields {
  readonly name: string;
  readonly host: string;
  readonly kind: string;
  readonly os: OperatingSystem;
  readonly tags: readonly string[];
  readonly ssh: {
    readonly enabled: boolean;
    readonly port: number;
    readonly username: string;
    readonly auth: SshAuth;
  };
}

export interface SshSettingsInput {
  readonly enabled: boolean;
  readonly port?: number;
  readonly username: string;
  readonly auth: SshAuth;
}

export interface CreateDeviceInput {
  readonly siteId: string;
  readonly name: string;
  readonly host: string;
  readonly kind?: string;
  readonly os?: OperatingSystem;
  readonly tags?: readonly string[];
  readonly ssh?: SshSettingsInput;
  readonly addDefaultCheck?: boolean;
}

export interface UpdateDeviceInput {
  readonly siteId?: string;
  readonly name?: string;
  readonly host?: string;
  readonly kind?: string;
  readonly os?: OperatingSystem;
  readonly tags?: readonly string[];
  readonly maintenance?: boolean;
  readonly ssh?: SshSettingsInput;
}

/** `@oneOf`: exactly one member is present. */
export type SshCredentialInput =
  | { readonly password: string }
  | { readonly privateKey: { readonly key: string; readonly passphrase?: string } };

export interface ConnectionTestInput {
  readonly deviceId?: string;
  readonly host?: string;
  readonly ssh?: SshSettingsInput;
  readonly credential?: SshCredentialInput;
}

export interface ConnectionTestResult {
  readonly reachable: boolean;
  readonly authenticated: boolean;
  readonly hostKey?: HostKey | null;
  readonly hostKeyChanged: boolean;
  readonly message?: string | null;
}

/** A site the Add device form can place the device in. */
export interface SiteOption {
  readonly id: string;
  readonly name: string;
}
