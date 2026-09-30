// BlindHop mixnet frame codec.
//
// Matches MixnetMessage::to_bytes / from_bytes in blindhop-common:
//   [type: u8][correlation_id: u64 big-endian][payload bytes]
// Low nibble of type: 0x01 = request, 0x02 = response.
// Bit 0x40 (requests): sender accepts compressed responses.
// Bit 0x80: payload is deflate-raw compressed.

export const FRAME_TYPE_REQUEST = 0x01;
export const FRAME_TYPE_RESPONSE = 0x02;
const FRAME_TYPE_MASK = 0x0f;
const FLAG_ACCEPTS_COMPRESSION = 0x40;
const FLAG_COMPRESSED = 0x80;
const FRAME_HEADER_LEN = 9;

// Largest payload a compressed frame may expand to (matches
// MAX_DECOMPRESSED_BYTES in Rust). Guards against decompression bombs.
export const MAX_DECOMPRESSED_BYTES = 8 * 1024 * 1024;

/** Whether this browser can read compressed responses. */
export const SUPPORTS_COMPRESSION = (() => {
    try {
        new DecompressionStream('deflate-raw');
        return true;
    } catch {
        return false;
    }
})();

/**
 * Encode a MixnetMessage frame.
 * @param {number} type - FRAME_TYPE_REQUEST or FRAME_TYPE_RESPONSE.
 * @param {number} correlationId
 * @param {Uint8Array} payload
 * @param {boolean} [acceptsCompression] - requests only: ask for compressed replies.
 * @returns {Uint8Array}
 */
export function encodeFrame(type, correlationId, payload, acceptsCompression = false) {
    const frame = new Uint8Array(FRAME_HEADER_LEN + payload.length);
    frame[0] = type === FRAME_TYPE_REQUEST && acceptsCompression ? type | FLAG_ACCEPTS_COMPRESSION : type;
    new DataView(frame.buffer).setBigUint64(1, BigInt(correlationId), false);
    frame.set(payload, FRAME_HEADER_LEN);
    return frame;
}

/**
 * Decode a MixnetMessage frame header. A compressed payload is returned as
 * is (`compressed: true`); pass it to inflateRaw().
 * @param {Uint8Array} bytes
 * @returns {{ type: number, correlationId: number, payload: Uint8Array, compressed: boolean } | null}
 *   null if the bytes are not a valid frame.
 */
export function decodeFrame(bytes) {
    if (!(bytes instanceof Uint8Array) || bytes.length < FRAME_HEADER_LEN) return null;
    const typeByte = bytes[0];
    const unknownBits = typeByte & ~(FRAME_TYPE_MASK | FLAG_ACCEPTS_COMPRESSION | FLAG_COMPRESSED) & 0xff;
    const type = typeByte & FRAME_TYPE_MASK;
    if (unknownBits || (type !== FRAME_TYPE_REQUEST && type !== FRAME_TYPE_RESPONSE)) return null;
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    return {
        type,
        correlationId: Number(view.getBigUint64(1, false)),
        payload: bytes.subarray(FRAME_HEADER_LEN),
        compressed: (typeByte & FLAG_COMPRESSED) !== 0,
    };
}

/**
 * Decompress a deflate-raw payload, refusing to expand past maxBytes.
 * @param {Uint8Array} bytes
 * @param {number} [maxBytes]
 * @returns {Promise<Uint8Array>}
 */
export async function inflateRaw(bytes, maxBytes = MAX_DECOMPRESSED_BYTES) {
    const reader = new Blob([bytes]).stream()
        .pipeThrough(new DecompressionStream('deflate-raw'))
        .getReader();
    const chunks = [];
    let total = 0;
    for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        total += value.length;
        if (total > maxBytes) {
            await reader.cancel();
            throw new Error(`Decompressed payload exceeds ${maxBytes} bytes`);
        }
        chunks.push(value);
    }
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
        out.set(chunk, offset);
        offset += chunk.length;
    }
    return out;
}
