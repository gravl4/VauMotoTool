#!/usr/bin/env python3
"""
UDP Sample Sender
Sends 44-byte data packets to UDP port 55512.

Packet layout (44 bytes, big-endian / MSB first):
  [0:2]   preamble  – 0x55 0x33
  [2:6]   uint32    – timestamp µs, wrapping counter
  [6:38]  int16×16  – analog channels 0..15 (sinusoids 1–16 Hz, ±1000 LSB)
  [38:42] uint32    – digital channels 0..31 (square waves 1–20 Hz)
  [42:44] uint16    – checksum over bytes [0..41]

Calculate CS from 0x5533 as C function: 
uint16_t checksum(uint8_t *ptr, uint16_t len)//, uint8_t type) { uint32_t sum = 0;

    /*if(type == 1) // это для UDP
    {
        sum+=IP_UDP;
        sum+=len-8;
    }  */

    while(len > 0)
    {
        sum += (uint16_t) (((uint32_t)*ptr<<8) |*(ptr+1));
        ptr+=2; //переходим еще на 16 бит
        len-=2; //
    }
    if (len) sum+=((uint32_t)*ptr)<<8;
    while(sum>>16) sum = (uint16_t)sum+(sum>>16);
    return ~((uint16_t)sum); //сдесь мы переобразовали к виду big endian и сделали побитовую инверсию
}
"""

import socket
import struct
import time
import math
import argparse

# ═══════════════════════════════ Constants ═══════════════════════════════════

HOST         = '127.0.0.1'
PORT         = 55512
DEFAULT_RATE = 1_000        # Hz  (1 ms / sample)

PREAMBLE     = bytes([0x55, 0x33])

NUM_ANALOG   = 16           # channels 0 … 15
NUM_DIGITAL  = 32           # bits 0 … 31
AMPLITUDE    = 1000         # int16 peak amplitude  (−1000 … 0 … +1000)

# Evenly spaced frequencies: 1 Hz … 20 Hz
ANALOG_FREQS  = [1.0 + i * 19.0 / (NUM_ANALOG  - 1) for i in range(NUM_ANALOG)]
DIGITAL_FREQS = [1.0 + i * 19.0 / (NUM_DIGITAL - 1) for i in range(NUM_DIGITAL)]

PACKET_SIZE   = 2 + 4 + NUM_ANALOG * 2 + 4 + 2   # must be 44

# ═══════════════════════════════ Checksum ════════════════════════════════════

def calc_checksum(data: bytes) -> int:
    """
    Port of C function:
        uint16_t checksum(uint8_t *ptr, uint16_t len)

    Sums data as big-endian 16-bit words, folds carry bits,
    returns bitwise NOT of 16-bit result.

    Applied to bytes [0..41]  (everything except the 2-byte CS field).
    """
    acc = 0
    idx = 0
    rem = len(data)

    while rem > 1:
        acc += (data[idx] << 8) | data[idx + 1]
        idx += 2
        rem -= 2

    if rem:                             # odd trailing byte
        acc += data[idx] << 8

    # Fold 32-bit accumulator into 16 bits
    while acc >> 16:
        acc = (acc & 0xFFFF) + (acc >> 16)

    return (~acc) & 0xFFFF

# ═══════════════════════════════ Packet builder ══════════════════════════════

def build_packet(timestamp_us: int, t_sec: float) -> bytes:
    """
    Assemble one 44-byte sample packet (all multi-byte fields MSB first).

    Structure:
        [0:2]   preamble  0x55 0x33
        [2:6]   timestamp µs  uint32
        [6:38]  analog ch 0..15  int16 × 16
        [38:42] digital word     uint32
        [42:44] checksum         uint16
    """
    buf = bytearray()

    # ── 2 bytes: preamble ─────────────────────────────────────────────────────
    buf += PREAMBLE

    # ── 4 bytes: timestamp µs, uint32, MSB first ─────────────────────────────
    buf += struct.pack('>I', timestamp_us & 0xFFFF_FFFF)

    # ── 32 bytes: analog channels 0..15, int16, MSB first ────────────────────
    for freq in ANALOG_FREQS:
        sample = int(round(AMPLITUDE * math.sin(2.0 * math.pi * freq * t_sec)))
        sample = max(-AMPLITUDE, min(AMPLITUDE, sample))
        buf += struct.pack('>h', sample)

    # ── 4 bytes: digital channels 0..31, uint32, MSB first ───────────────────
    # Square wave: bit = 1 while sin(2π·f·t) ≥ 0
    digital = 0
    for bit, freq in enumerate(DIGITAL_FREQS):
        if math.sin(2.0 * math.pi * freq * t_sec) >= 0.0:
            digital |= (1 << bit)
    buf += struct.pack('>I', digital)

    # ── 2 bytes: checksum over bytes [0..41], uint16, MSB first ──────────────
    cs = calc_checksum(bytes(buf))      # covers preamble + all data
    buf += struct.pack('>H', cs)

    assert len(buf) == PACKET_SIZE, f"Packet size error: {len(buf)} != {PACKET_SIZE}"
    return bytes(buf)

# ═══════════════════════════════ Main loop ═══════════════════════════════════

def main() -> None:
    parser = argparse.ArgumentParser(
        description='Send 44-byte UDP sample packets (preamble 0x55 0x33).')
    parser.add_argument('--host', default=HOST,
                        help=f'Destination host  (default: {HOST})')
    parser.add_argument('--port', type=int, default=PORT,
                        help=f'Destination port  (default: {PORT})')
    parser.add_argument('--rate', type=int, default=DEFAULT_RATE,
                        help=f'Sample rate Hz    (default: {DEFAULT_RATE})')
    args = parser.parse_args()

    interval_s = 1.0 / args.rate
    dt_us      = round(1_000_000 / args.rate)

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    timestamp_us = 0
    sent         = 0
    t0           = time.perf_counter()

    # ── Startup banner ────────────────────────────────────────────────────────
    freq_a = ", ".join(f"{f:.2f}" for f in ANALOG_FREQS)
    print(f"{'─'*64}")
    print(f"  UDP sender  →  {args.host}:{args.port}")
    print(f"  Sample rate : {args.rate} Hz  ({interval_s*1000:.3f} ms/sample)")
    print(f"  Packet size : {PACKET_SIZE} bytes  |  preamble: 0x55 0x33")
    print(f"  Analog  ch  : {NUM_ANALOG} ch  freqs (Hz): {freq_a}")
    print(f"  Digital ch  : {NUM_DIGITAL} bits  freqs: "
          f"{DIGITAL_FREQS[0]:.2f} … {DIGITAL_FREQS[-1]:.2f} Hz")
    print(f"{'─'*64}")
    print("  Press Ctrl-C to stop.\n")

    try:
        while True:
            t_sec  = timestamp_us * 1e-6
            packet = build_packet(timestamp_us, t_sec)
            sock.sendto(packet, (args.host, args.port))

            timestamp_us = (timestamp_us + dt_us) & 0xFFFF_FFFF
            sent        += 1

            # Print stats once per second
            if sent % args.rate == 0:
                elapsed  = time.perf_counter() - t0
                hex_head = packet[:6].hex(' ').upper()      # preamble + ts
                ch0      = struct.unpack('>h', packet[6:8])[0]
                digi     = struct.unpack('>I', packet[38:42])[0]
                cs_rx    = struct.unpack('>H', packet[42:44])[0]
                print(f"  pkt={sent:>8} | t={elapsed:6.1f}s | "
                      f"[{hex_head}] | ch0={ch0:+5d} | "
                      f"digi=0x{digi:08X} | cs=0x{cs_rx:04X}")

            # ── Precise inter-packet sleep (deadline-based, no drift) ─────────
            deadline = t0 + sent * interval_s
            wait     = deadline - time.perf_counter()
            if wait > 0:
                time.sleep(wait)

    except KeyboardInterrupt:
        elapsed = time.perf_counter() - t0
        print(f"\n  Stopped. Sent {sent} packets in {elapsed:.2f}s "
              f"({sent/elapsed:.1f} Hz actual)")
    finally:
        sock.close()


if __name__ == '__main__':
    main()

