// BlindHop mixnet frame codec.
//
// Matches MixnetMessage::to_bytes / from_bytes in blindhop-common:
//   [type: u8][correlation_id: u64 big-endian][payload bytes]
//   type 0x01 = request, 0x02 = response

export const FRAME_TYPE_REQUEST = 0x01;
export const FRAME_TYPE_RESPONSE = 0x02;
const FRAME_HEADER_LEN = 9;

/**
 * Encode a MixnetMessage frame.
 * @param {number} type - FRAME_TYPE_REQUEST or FRAME_TYPE_RESPONSE.
 * @param {number} correlationId
 * @param {Uint8Array} payload
 * @returns {Uint8Array}
 */
export function encodeFrame(type, correlationId, payload) {
    const frame = new Uint8Array(FRAME_HEADER_LEN + payload.length);
    frame[0] = type;
    new DataView(frame.buffer).setBigUint64(1, BigInt(correlationId), false);
    frame.set(payload, FRAME_HEADER_LEN);
    return frame;
}

/**
 * Decode a MixnetMessage frame.
 * @param {Uint8Array} bytes
 * @returns {{ type: number, correlationId: number, payload: Uint8Array } | null}
 *   null if the bytes are not a valid frame.
 */
export function decodeFrame(bytes) {
    if (!(bytes instanceof Uint8Array) || bytes.length < FRAME_HEADER_LEN) return null;
    const type = bytes[0];
    if (type !== FRAME_TYPE_REQUEST && type !== FRAME_TYPE_RESPONSE) return null;
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    return {
        type,
        correlationId: Number(view.getBigUint64(1, false)),
        payload: bytes.subarray(FRAME_HEADER_LEN),
    };
}
