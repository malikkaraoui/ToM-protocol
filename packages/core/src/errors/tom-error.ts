export type TomErrorCode =
  | 'TRANSPORT_FAILED'
  | 'PEER_UNREACHABLE'
  | 'SIGNALING_TIMEOUT'
  | 'INVALID_ENVELOPE'
  | 'IDENTITY_MISSING'
  | 'RELAY_REJECTED'
  | 'CRYPTO_FAILED';

export class TomError extends Error {
  readonly code: TomErrorCode;
  readonly context?: Record<string, unknown>;

  constructor(code: TomErrorCode, message: string, context?: Record<string, unknown>) {
    super(message);
    this.name = 'TomError';
    this.code = code;
    this.context = context;
  }
}
