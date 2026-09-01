/**
 * @format
 */

import React from 'react';
import ReactTestRenderer from 'react-test-renderer';
import App from '../App';

jest.mock('@dr.pogodin/react-native-fs', () => ({
  DocumentDirectoryPath: '/tmp',
  exists: jest.fn().mockResolvedValue(true),
  mkdir: jest.fn().mockResolvedValue(undefined),
}));

jest.mock('react-native-nitro-tor', () => ({
  Tor: {
    daemon: {
      subscribe: jest.fn(() => jest.fn()),
      start: jest.fn().mockResolvedValue({
        state: 'running',
        socksAddress: '127.0.0.1:9050',
        controlAddress: '127.0.0.1:9051',
        connectivity: { network: 'up', circuitEstablished: true },
      }),
      stop: jest.fn().mockResolvedValue(undefined),
    },
    hiddenServices: {
      create: jest.fn().mockResolvedValue({
        onionAddress: 'example.onion',
        privateKey: new Uint8Array(64),
      }),
    },
    http: { request: jest.fn() },
  },
}));

test('renders correctly', async () => {
  let renderer: ReactTestRenderer.ReactTestRenderer;
  await ReactTestRenderer.act(() => {
    renderer = ReactTestRenderer.create(<App />);
  });
  await ReactTestRenderer.act(() => {
    renderer.unmount();
  });
});
