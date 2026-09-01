# react-native-nitro-tor

Run a process-wide Tor daemon, make HTTP requests through it, and manage ephemeral v3 onion services from React Native. The native bridge is built with [Craby](https://craby.rs).

> `1.0.0-rc.1` is a breaking release. The pre-1.0 `RnTor` methods and numeric status values have been removed.

## Installation

```bash
yarn add react-native-nitro-tor
cd ios && pod install
```

The library supports iOS and Android. It includes arm64, x86_64, x86, and armeabi-v7a Android binaries and an iOS XCFramework.

## Start and observe Tor

The app owns the data-directory choice. Starting resolves only after Tor reaches 100% bootstrap.

```ts
import { Tor, TorError, type TorStatus } from 'react-native-nitro-tor';

const unsubscribe = Tor.daemon.subscribe((status: TorStatus) => {
  if (status.state === 'starting') {
    console.log(status.bootstrap.progress, status.bootstrap.summary);
  }
});

try {
  const running = await Tor.daemon.start({
    dataDirectory: '/path/chosen/by/the/app',
    socksPort: 9050,
    bootstrapTimeoutMs: 60_000,
  });
  console.log(running.socksAddress, running.connectivity);
} catch (error) {
  if (error instanceof TorError) {
    console.error(error.code, error.message);
  }
}

unsubscribe();
await Tor.daemon.stop();
```

An identical concurrent or repeated `start` shares the current daemon. A different configuration rejects with `CONFIG_CONFLICT`. `stop` is idempotent and cancels startup and in-flight HTTP requests.

### Status

`Tor.daemon.getStatus()` and subscriptions return a discriminated union:

```ts
type TorStatus =
  | { state: 'stopped' }
  | {
      state: 'starting';
      bootstrap: {
        progress: number;
        tag: string;
        summary: string;
        warning?: string;
      };
    }
  | {
      state: 'running';
      socksAddress: string;
      connectivity: {
        network: 'up' | 'down' | 'unknown';
        circuitEstablished: boolean;
      };
    }
  | { state: 'stopping' }
  | { state: 'failed'; error: { code: string; message: string } };
```

The facade refreshes native status when the app returns to the foreground. Listener failures are isolated from other listeners.

Request a new circuit identity after Tor is running:

```ts
await Tor.daemon.requestNewIdentity();
```

Tor may rate-limit how quickly a new identity takes effect. The promise means the control command was accepted.

## HTTP over Tor

HTTP status errors resolve normally. Transport failures, timeouts, and daemon shutdown reject with `TorError`.

```ts
const response = await Tor.http.request({
  url: 'http://example.onion/resource',
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ hello: 'tor' }),
  timeoutMs: 30_000,
});

console.log(response.statusCode);
console.log(response.headers); // lowercase names, string[] values
console.log(response.body);
```

Supported methods are `GET`, `POST`, `PUT`, `DELETE`, `HEAD`, and `OPTIONS`. Set `allowInvalidCertificates: true` only when the caller explicitly intends to accept an invalid TLS chain.

## Ephemeral onion services

Private keys are caller-owned. Omitting the key creates and returns a new 64-byte key; pass it again to recreate the same address in a future daemon session.

```ts
const service = await Tor.hiddenServices.create({
  virtualPort: 80,
  targetPort: 8080,
});

console.log(service.onionAddress);
persistSecurely(service.privateKey);

await Tor.hiddenServices.remove(service.onionAddress);
```

`remove` is idempotent for an address that is not active in this process. Active services close when the daemon stops. The library does not persist or automatically restore private keys.

## Errors

All operational failures reject with `TorError`, which exposes a stable `code` and a human-readable `message`. Common codes include:

- `INVALID_CONFIG`, `CONFIG_CONFLICT`, `BOOTSTRAP_TIMEOUT`, `TOR_START_FAILED`
- `NOT_RUNNING`, `TOR_STOPPED`, `CONTROL_CONNECTION_FAILED`
- `HTTP_TIMEOUT`, `HTTP_TRANSPORT_ERROR`
- `INVALID_PRIVATE_KEY`, `HIDDEN_SERVICE_EXISTS`, `HIDDEN_SERVICE_ERROR`

## Running the example

```bash
yarn install
yarn crabygen codegen
yarn example start
```

Build the iOS example with Xcode or:

```bash
xcodebuild build \
  -workspace example/ios/ReactNativeNitroTorExample.xcworkspace \
  -scheme ReactNativeNitroTorExample \
  -configuration Debug \
  -sdk iphonesimulator \
  ARCHS=arm64
```

## License

MIT
