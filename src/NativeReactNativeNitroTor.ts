import type { NativeModule, Signal } from 'craby-modules';
import { NativeModuleRegistry } from 'craby-modules';

export interface NativeTorConfig {
  data_directory: string;
  socks_port: number;
  bootstrap_timeout_ms: number;
}

export interface NativeHttpRequest {
  url: string;
  method: string;
  headers_json: string;
  body: string;
  timeout_ms: number;
  allow_invalid_certificates: boolean;
}

export interface NativeHiddenServiceOptions {
  virtual_port: number;
  target_port: number;
  private_key: ArrayBuffer;
}

export interface NativeHiddenService {
  onion_address: string;
  private_key: ArrayBuffer;
}

interface Spec extends NativeModule {
  start(config: NativeTorConfig): Promise<string>;
  stop(): Promise<void>;
  getStatus(): Promise<string>;
  requestNewIdentity(): Promise<void>;
  httpRequest(request: NativeHttpRequest): Promise<string>;
  createHiddenService(options: NativeHiddenServiceOptions): Promise<NativeHiddenService>;
  removeHiddenService(onionAddress: string): Promise<void>;
  readonly onStatusChange: Signal<string>;
}

export default NativeModuleRegistry.getEnforcing<Spec>('ReactNativeNitroTor');
