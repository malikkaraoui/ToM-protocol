/** Valid ACK types for runtime validation */
export const VALID_ACK_TYPES: ('relay-forwarded' | 'recipient-received' | 'recipient-read')[] = ['relay-forwarded', 'recipient-received', 'recipient-read'];

/** Max age for ACK anti-replay cache (5 minutes) */
export const ACK_REPLAY_CACHE_TTL_MS = 5 * 60 * 1000;

/** Max entries in ACK anti-replay cache */
export const ACK_REPLAY_CACHE_MAX_SIZE = 5000;

/** Max age for message deduplication cache (10 minutes) */
export const MESSAGE_DEDUP_CACHE_TTL_MS = 10 * 60 * 1000;

/** Max entries in message deduplication cache */
export const MESSAGE_DEDUP_CACHE_MAX_SIZE = 10000;

/**
 * Check if a message has already been received (deduplication).
 * Returns true if duplicate, false if new message.
 *
 * Uses composite key (messageId:from) to prevent collision when
 * different senders happen to generate the same messageId.
 */
export function checkMessageDuplicate(
  messageId: string,
  from: string,
  receivedMessages: Map<string, number>,
  ttlMs: number,
  maxSize: number,
  onDuplicateMessage?: (messageId: string, from: string) => void,
): boolean {
  const now = Date.now();
  // Composite key prevents collision between different senders
  const dedupKey = `${messageId}:${from}`;

  // Clean up old entries periodically (trigger at 50% capacity)
  if (receivedMessages.size > maxSize / 2) {
    for (const [key, timestamp] of receivedMessages) {
      if (now - timestamp > ttlMs) {
        receivedMessages.delete(key);
      }
    }
  }

  // Evict oldest if still over limit
  if (receivedMessages.size >= maxSize) {
    const firstKey = receivedMessages.keys().next().value;
    if (firstKey) receivedMessages.delete(firstKey);
  }

  // Check if already received
  if (receivedMessages.has(dedupKey)) {
    onDuplicateMessage?.(messageId, from);
    return true;
  }

  // Record as received
  receivedMessages.set(dedupKey, now);
  return false;
}

/**
 * Check if an ACK/receipt has been seen before (anti-replay).
 * Also cleans up old entries to prevent memory leaks.
 */
export function checkAndRecordSeen(
  cache: Map<string, number>,
  key: string,
  ttlMs: number,
  maxSize: number,
): boolean {
  const now = Date.now();

  // Clean up old entries periodically
  if (cache.size > maxSize / 2) {
    for (const [k, timestamp] of cache) {
      if (now - timestamp > ttlMs) {
        cache.delete(k);
      }
    }
  }

  // Evict oldest if still over limit
  if (cache.size >= maxSize) {
    const firstKey = cache.keys().next().value;
    if (firstKey) cache.delete(firstKey);
  }

  // Check if already seen
  if (cache.has(key)) {
    return true; // Replay detected
  }

  // Record as seen
  cache.set(key, now);
  return false;
}
