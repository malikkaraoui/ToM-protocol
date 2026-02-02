import nacl from 'tweetnacl';

export interface NodeIdentity {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}

export type NodeId = string;

export function generateKeypair(): NodeIdentity {
  const pair = nacl.sign.keyPair();
  return { publicKey: pair.publicKey, secretKey: pair.secretKey };
}

export function signData(secretKey: Uint8Array, data: Uint8Array): Uint8Array {
  return nacl.sign.detached(data, secretKey);
}

export function verifySignature(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean {
  return nacl.sign.detached.verify(data, signature, publicKey);
}

export function publicKeyToNodeId(publicKey: Uint8Array): NodeId {
  return Array.from(publicKey)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}
