/**
 * Group Module (Story 4.6 - Group Messaging)
 *
 * Provides group chat functionality using relays as "temporary hubs".
 * Each group has a designated relay that handles message fanout.
 */

export * from './group-types.js';
export * from './group-manager.js';
export * from './group-hub.js';
export * from './group-security.js';
export * from './hub-election.js';
