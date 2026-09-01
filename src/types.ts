export interface TorConfig {
  dataDirectory: string;
  socksPort: number;
  bootstrapTimeoutMs: number;
}

export interface BootstrapStatus {
  progress: number;
  tag: string;
  summary: string;
  warning?: string;
}

export interface StoppedTorStatus {
  state: 'stopped';
}

export interface StartingTorStatus {
  state: 'starting';
  bootstrap: BootstrapStatus;
}

export interface RunningTorStatus {
  state: 'running';
  socksAddress: string;
  connectivity: {
    network: 'up' | 'down' | 'unknown';
    circuitEstablished: boolean;
  };
}

export interface StoppingTorStatus {
  state: 'stopping';
}

export interface FailedTorStatus {
  state: 'failed';
  error: {
    code: TorErrorCode;
    message: string;
  };
}

export type TorStatus = StoppedTorStatus | StartingTorStatus | RunningTorStatus | StoppingTorStatus | FailedTorStatus;

export type TorErrorCode =
  | 'INVALID_CONFIG'
  | 'CONFIG_CONFLICT'
  | 'TOR_START_FAILED'
  | 'BOOTSTRAP_TIMEOUT'
  | 'TOR_STOPPED'
  | 'TOR_STOP_FAILED'
  | 'NOT_RUNNING'
  | 'CONTROL_CONNECTION_FAILED'
  | 'NEW_IDENTITY_FAILED'
  | 'INVALID_REQUEST'
  | 'HTTP_TIMEOUT'
  | 'HTTP_TRANSPORT_ERROR'
  | 'INVALID_HIDDEN_SERVICE'
  | 'INVALID_PRIVATE_KEY'
  | 'HIDDEN_SERVICE_EXISTS'
  | 'HIDDEN_SERVICE_ERROR'
  | 'INTERNAL_ERROR';

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'HEAD' | 'OPTIONS';

export interface TorHttpRequest {
  url: string;
  method?: HttpMethod;
  headers?: Record<string, string>;
  body?: string;
  timeoutMs?: number;
  allowInvalidCertificates?: boolean;
}

export interface TorHttpResponse {
  statusCode: number;
  headers: Record<string, string[]>;
  body: string;
}

export interface HiddenServiceOptions {
  virtualPort: number;
  targetPort: number;
  privateKey?: Uint8Array;
}

export interface HiddenService {
  onionAddress: string;
  privateKey: Uint8Array;
}

export type TorStatusListener = (status: TorStatus) => void;
