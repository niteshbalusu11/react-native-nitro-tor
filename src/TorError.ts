import type { TorErrorCode } from './types';

export class TorError extends Error {
  readonly code: TorErrorCode;

  constructor(code: TorErrorCode, message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = 'TorError';
    this.code = code;
  }
}

export function toTorError(error: unknown): TorError {
  if (error instanceof TorError) {
    return error;
  }
  const message = error instanceof Error ? error.message : String(error);
  const marker = 'TOR_ERROR:';
  const markerIndex = message.indexOf(marker);
  if (markerIndex >= 0) {
    try {
      const payload = JSON.parse(message.slice(markerIndex + marker.length)) as {
        code?: TorErrorCode;
        message?: string;
      };
      if (payload.code && payload.message) {
        return new TorError(payload.code, payload.message, { cause: error });
      }
    } catch {
      // Fall through to a stable error when native returned malformed payload.
    }
  }
  return new TorError('INTERNAL_ERROR', message, { cause: error });
}
