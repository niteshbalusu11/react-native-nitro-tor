import { AppState, type NativeEventSubscription } from 'react-native';

import NativeTor from './NativeReactNativeNitroTor';
import { TorError, toTorError } from './TorError';
import type {
  HiddenService,
  HiddenServiceOptions,
  RunningTorStatus,
  TorConfig,
  TorHttpRequest,
  TorHttpResponse,
  TorStatus,
  TorStatusListener,
} from './types';

const statusListeners = new Set<TorStatusListener>();
let removeNativeStatusListener: (() => void) | undefined;
let appStateSubscription: NativeEventSubscription | undefined;
let nativeStatusRevision = 0;
let latestRefreshId = 0;

function assertPositiveInteger(value: number, field: string, code: 'INVALID_CONFIG' | 'INVALID_REQUEST'): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new TorError(code, `${field} must be a positive integer`);
  }
}

function assertPort(value: number, field: string, code: 'INVALID_CONFIG' | 'INVALID_HIDDEN_SERVICE'): void {
  if (!Number.isInteger(value) || value < 1 || value > 65535) {
    throw new TorError(code, `${field} must be an integer from 1 through 65535`);
  }
}

function parseStatus(payload: string): TorStatus {
  let status: unknown;
  try {
    status = JSON.parse(payload);
  } catch (error) {
    throw new TorError('INTERNAL_ERROR', 'Native Tor returned invalid status JSON', { cause: error });
  }
  if (!status || typeof status !== 'object' || typeof (status as { state?: unknown }).state !== 'string') {
    throw new TorError('INTERNAL_ERROR', 'Native Tor returned an invalid status');
  }
  return status as TorStatus;
}

function dispatchStatus(status: TorStatus): void {
  for (const listener of statusListeners) {
    try {
      listener(status);
    } catch (error) {
      console.error('A Tor status listener threw an error', error);
    }
  }
}

async function refreshStatus(): Promise<TorStatus> {
  const refreshId = ++latestRefreshId;
  const revisionAtStart = nativeStatusRevision;
  try {
    const status = parseStatus(await NativeTor.getStatus());
    if (refreshId === latestRefreshId && revisionAtStart === nativeStatusRevision) {
      dispatchStatus(status);
    }
    return status;
  } catch (error) {
    throw toTorError(error);
  }
}

function ensureStatusObservers(): void {
  if (!removeNativeStatusListener) {
    removeNativeStatusListener = NativeTor.onStatusChange((payload) => {
      try {
        const status = parseStatus(payload);
        nativeStatusRevision += 1;
        dispatchStatus(status);
      } catch (error) {
        console.error('Unable to process a native Tor status update', error);
      }
    });
  }
  if (!appStateSubscription) {
    appStateSubscription = AppState.addEventListener('change', (state) => {
      if (state === 'active') {
        void refreshStatus().catch((error) => {
          console.error('Unable to refresh Tor status after entering the foreground', error);
        });
      }
    });
  }
}

function removeStatusObserversIfUnused(): void {
  if (statusListeners.size !== 0) {
    return;
  }
  removeNativeStatusListener?.();
  removeNativeStatusListener = undefined;
  appStateSubscription?.remove();
  appStateSubscription = undefined;
}

const daemon = {
  async start(config: TorConfig): Promise<RunningTorStatus> {
    if (!config.dataDirectory.trim()) {
      throw new TorError('INVALID_CONFIG', 'dataDirectory must not be empty');
    }
    assertPort(config.socksPort, 'socksPort', 'INVALID_CONFIG');
    assertPositiveInteger(config.bootstrapTimeoutMs, 'bootstrapTimeoutMs', 'INVALID_CONFIG');
    try {
      const status = parseStatus(
        await NativeTor.start({
          data_directory: config.dataDirectory,
          socks_port: config.socksPort,
          bootstrap_timeout_ms: config.bootstrapTimeoutMs,
        }),
      );
      if (status.state !== 'running') {
        throw new TorError('INTERNAL_ERROR', `Tor start resolved in the ${status.state} state`);
      }
      return status;
    } catch (error) {
      throw toTorError(error);
    }
  },

  async stop(): Promise<void> {
    try {
      await NativeTor.stop();
    } catch (error) {
      throw toTorError(error);
    }
  },

  async getStatus(): Promise<TorStatus> {
    try {
      return parseStatus(await NativeTor.getStatus());
    } catch (error) {
      throw toTorError(error);
    }
  },

  subscribe(listener: TorStatusListener): () => void {
    statusListeners.add(listener);
    ensureStatusObservers();
    void refreshStatus().catch((error) => {
      console.error('Unable to load the initial Tor status', error);
    });
    return () => {
      statusListeners.delete(listener);
      removeStatusObserversIfUnused();
    };
  },

  async requestNewIdentity(): Promise<void> {
    try {
      await NativeTor.requestNewIdentity();
    } catch (error) {
      throw toTorError(error);
    }
  },
};

const http = {
  async request(request: TorHttpRequest): Promise<TorHttpResponse> {
    if (!request.url.trim()) {
      throw new TorError('INVALID_REQUEST', 'url must not be empty');
    }
    const timeoutMs = request.timeoutMs ?? 30_000;
    assertPositiveInteger(timeoutMs, 'timeoutMs', 'INVALID_REQUEST');
    try {
      return JSON.parse(
        await NativeTor.httpRequest({
          url: request.url,
          method: request.method ?? 'GET',
          headers_json: JSON.stringify(request.headers ?? {}),
          body: request.body ?? '',
          timeout_ms: timeoutMs,
          allow_invalid_certificates: request.allowInvalidCertificates ?? false,
        }),
      ) as TorHttpResponse;
    } catch (error) {
      throw toTorError(error);
    }
  },
};

const hiddenServices = {
  async create(options: HiddenServiceOptions): Promise<HiddenService> {
    assertPort(options.virtualPort, 'virtualPort', 'INVALID_HIDDEN_SERVICE');
    assertPort(options.targetPort, 'targetPort', 'INVALID_HIDDEN_SERVICE');
    if (options.privateKey && options.privateKey.byteLength !== 64) {
      throw new TorError('INVALID_PRIVATE_KEY', 'privateKey must be exactly 64 bytes');
    }
    const key = options.privateKey;
    const privateKey = key ? Uint8Array.from(key).buffer : new ArrayBuffer(0);
    try {
      const result = await NativeTor.createHiddenService({
        virtual_port: options.virtualPort,
        target_port: options.targetPort,
        private_key: privateKey,
      });
      return {
        onionAddress: result.onion_address,
        privateKey: new Uint8Array(result.private_key),
      };
    } catch (error) {
      throw toTorError(error);
    }
  },

  async remove(onionAddress: string): Promise<void> {
    if (!onionAddress.trim()) {
      throw new TorError('INVALID_HIDDEN_SERVICE', 'onionAddress must not be empty');
    }
    try {
      await NativeTor.removeHiddenService(onionAddress);
    } catch (error) {
      throw toTorError(error);
    }
  },
};

export const Tor = Object.freeze({
  daemon: Object.freeze(daemon),
  http: Object.freeze(http),
  hiddenServices: Object.freeze(hiddenServices),
});
