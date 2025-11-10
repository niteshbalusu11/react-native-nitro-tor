import type { NativeModule } from "craby-modules";
import { NativeModuleRegistry } from "craby-modules";

export interface TorConfig {
  socks_port: number;
  data_dir: string;
  timeout_ms: number;
}

export interface HiddenServiceParams {
  port: number;
  target_port: number;
}

export interface StartTorParams {
  data_dir: string;
  socks_port: number;
  target_port: number;
  timeout_ms: number;
}

export interface StartTorResponse {
  is_success: boolean;
  onion_address: string;
  control: string;
  error_message: string;
}

export interface HiddenServiceResponse {
  is_success: boolean;
  onion_address: string;
  control: string;
}

export interface HttpGetParams {
  url: string;
  headers: string;
  timeout_ms: number;
}

export interface HttpPostParams {
  url: string;
  body: string;
  headers: string;
  timeout_ms: number;
}

export interface HttpPutParams {
  url: string;
  body: string;
  headers: string;
  timeout_ms: number;
}

export interface HttpDeleteParams {
  url: string;
  headers: string;
  timeout_ms: number;
}

export interface HttpResponse {
  status_code: number;
  body: string;
  error: string;
}

interface Spec extends NativeModule {
  // Initialize the Tor service
  initTorService(config: TorConfig): Promise<boolean>;

  // Create a new hidden service
  createHiddenService(
    params: HiddenServiceParams,
  ): Promise<HiddenServiceResponse>;

  // Start the Tor daemon with hidden service and control port
  startTorIfNotRunning(params: StartTorParams): Promise<StartTorResponse>;

  // Get the current service status
  getServiceStatus(): Promise<number>;

  // Delete an existing hidden service
  deleteHiddenService(onionAddress: string): Promise<boolean>;

  // Shutdown the Tor service
  shutdownService(): Promise<boolean>;

  // Http GET
  httpGet(params: HttpGetParams): Promise<HttpResponse>;

  // Http POST
  httpPost(params: HttpPostParams): Promise<HttpResponse>;

  // Http PUT
  httpPut(params: HttpPutParams): Promise<HttpResponse>;

  // Http Delete
  httpDelete(params: HttpDeleteParams): Promise<HttpResponse>;
}

export default NativeModuleRegistry.getEnforcing<Spec>("ReactNativeNitroTor");
