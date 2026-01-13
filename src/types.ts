// Forever File - TypeScript Types
// These interfaces match the Rust event payloads exactly

/**
 * Emitted when a transfer ticket is generated (sender side)
 * Event: forever-file://ticket-generated
 */
export interface TicketGeneratedEvent {
  ticket: string;
}

/**
 * Emitted during file transfer to report progress
 * Event: forever-file://progress
 */
export interface ProgressEvent {
  sent: number;
  total: number;
}

/**
 * Emitted when transfer completes (either success or handled failure)
 * Event: forever-file://complete
 */
export interface TransferCompleteEvent {
  success: boolean;
  message: string;
}

/**
 * Emitted when an error occurs during transfer
 * Event: forever-file://error
 */
export interface ErrorEvent {
  message: string;
}

/**
 * All possible event names
 */
export type ForeverFileEvent =
  | 'forever-file://ticket-generated'
  | 'forever-file://progress'
  | 'forever-file://complete'
  | 'forever-file://error';
