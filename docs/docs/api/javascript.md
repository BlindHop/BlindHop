---
sidebar_position: 4
title: JavaScript API
---

# JavaScript / TypeScript API

The `@blindhop/client` npm package provides a drop-in replacement for smoldot with built-in mixnet privacy.

## Installation

```bash
npm install @blindhop/client
```

## API Reference

### `BlindHop.start(options)`

Start a BlindHop-wrapped smoldot client.

```typescript
interface BlindHopOptions {
  chainSpec: string;           // Chain specification JSON
  hopCount?: number;           // 1–5, default: 3
  privacyMode?: 'required' | 'best-effort';  // default: 'required'
  coverTrafficRate?: number;   // packets/sec, default: 1.0
  delayParameter?: number;     // mean delay ms, default: 500
  minAnonymitySet?: number;    // minimum mixnodes, default: 30
}

const client = await BlindHop.start(options);
```

### `client.sendJsonRpc(request)`

Send a JSON-RPC request through the mixnet. Same interface as smoldot.

```typescript
const response = await client.sendJsonRpc(
  '{"jsonrpc":"2.0","id":1,"method":"chain_getBlockHash","params":[]}'
);
```

### `client.setHopCount(n)`

Dynamically adjust the number of mixnet hops.

### `client.setCoverRate(lambda)`

Adjust the cover traffic generation rate.

### `client.anonymityMetrics()`

```typescript
interface AnonymityMetrics {
  totalMixnodes: number;
  validatorMixnodes: number;
  standaloneMixnodes: number;
  status: 'active' | 'degraded' | 'fallback';
}
```

### `client.proofStatus()`

```typescript
interface ProofStatus {
  rootHash: string | null;      // Blake3 hash hex
  verified: boolean;
  proofCount: number;
}
```

### Events

```typescript
client.on('privacy-degraded', (metrics) => {
  console.warn('Privacy degraded:', metrics);
});

client.on('privacy-restored', (metrics) => {
  console.log('Privacy restored:', metrics);
});

client.on('proof-verified', (proof) => {
  console.log('Root proof verified:', proof.rootHash);
});
```

### `client.terminate()`

Gracefully shut down the client and stop cover traffic.

```typescript
await client.terminate();
```
