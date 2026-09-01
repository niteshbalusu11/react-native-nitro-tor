const mockNative = {
  start: jest.fn(),
  stop: jest.fn(),
  getStatus: jest.fn(),
  requestNewIdentity: jest.fn(),
  httpRequest: jest.fn(),
  createHiddenService: jest.fn(),
  removeHiddenService: jest.fn(),
  onStatusChange: jest.fn(),
};

let mockAppStateListener: ((state: string) => void) | undefined;

jest.mock('craby-modules', () => ({
  NativeModuleRegistry: {
    getEnforcing: () => mockNative,
  },
}));

jest.mock('react-native', () => ({
  AppState: {
    addEventListener: jest.fn((_event, listener) => {
      mockAppStateListener = listener;
      return { remove: jest.fn() };
    }),
  },
}));

import type { TorError as TorErrorType } from '../../src';

const { Tor } = require('../../src') as typeof import('../../src');

describe('Tor facade', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockNative.getStatus.mockResolvedValue('{"state":"stopped"}');
    mockNative.onStatusChange.mockReturnValue(jest.fn());
  });

  it('maps the public start config and requires a running result', async () => {
    mockNative.start.mockResolvedValue(
      '{"state":"running","socksAddress":"127.0.0.1:19050","controlAddress":"127.0.0.1:19051","connectivity":{"network":"up","circuitEstablished":true}}',
    );

    await expect(
      Tor.daemon.start({
        dataDirectory: '/tmp/tor',
        socksPort: 19050,
        bootstrapTimeoutMs: 45_000,
      }),
    ).resolves.toMatchObject({
      state: 'running',
      socksAddress: '127.0.0.1:19050',
      controlAddress: '127.0.0.1:19051',
    });
    expect(mockNative.start).toHaveBeenCalledWith({
      data_directory: '/tmp/tor',
      socks_port: 19050,
      bootstrap_timeout_ms: 45_000,
    });
  });

  it('turns native error payloads into typed TorError instances', async () => {
    mockNative.requestNewIdentity.mockRejectedValue(
      new Error('TOR_ERROR:{"code":"NOT_RUNNING","message":"Tor is not running"}'),
    );

    await expect(Tor.daemon.requestNewIdentity()).rejects.toEqual(
      expect.objectContaining<TorErrorType>({
        name: 'TorError',
        code: 'NOT_RUNNING',
        message: 'Tor is not running',
      }),
    );
  });

  it('keeps HTTP error status responses in the resolved path', async () => {
    mockNative.httpRequest.mockResolvedValue(
      '{"statusCode":404,"headers":{"content-type":["text/plain"]},"body":"missing"}',
    );

    await expect(Tor.http.request({ url: 'http://example.onion/missing' })).resolves.toEqual({
      statusCode: 404,
      headers: { 'content-type': ['text/plain'] },
      body: 'missing',
    });
  });

  it('copies hidden-service keys across the ArrayBuffer boundary', async () => {
    const privateKey = Uint8Array.from({ length: 64 }, (_, index) => index);
    mockNative.createHiddenService.mockResolvedValue({
      onion_address: 'example.onion',
      private_key: privateKey.buffer,
    });

    const hiddenService = await Tor.hiddenServices.create({
      virtualPort: 80,
      targetPort: 8080,
      privateKey,
    });

    expect(hiddenService.onionAddress).toBe('example.onion');
    expect(hiddenService.privateKey).toEqual(privateKey);
    expect(mockNative.createHiddenService.mock.calls[0][0].private_key).not.toBe(privateKey.buffer);
  });

  it('isolates listeners and refreshes status when the app becomes active', async () => {
    const consoleError = jest.spyOn(console, 'error').mockImplementation(() => undefined);
    const first = jest.fn(() => {
      throw new Error('listener failed');
    });
    const second = jest.fn();
    const unsubscribeFirst = Tor.daemon.subscribe(first);
    const unsubscribeSecond = Tor.daemon.subscribe(second);
    await Promise.resolve();

    const nativeListener = mockNative.onStatusChange.mock.calls[0][0] as (payload: string) => void;
    nativeListener('{"state":"stopping"}');
    expect(second).toHaveBeenCalledWith({ state: 'stopping' });

    mockNative.getStatus.mockResolvedValue('{"state":"running","socksAddress":"127.0.0.1:19050","controlAddress":"127.0.0.1:19051","connectivity":{"network":"unknown","circuitEstablished":false}}');
    mockAppStateListener?.('active');
    await Promise.resolve();
    await Promise.resolve();
    expect(mockNative.getStatus).toHaveBeenCalled();

    unsubscribeFirst();
    unsubscribeSecond();
    consoleError.mockRestore();
  });
});
