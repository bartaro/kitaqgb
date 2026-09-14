#pragma once

// Polling driver for the physical Nintendo DMG-07 Four Player Adapter.
//
// The adapter supplies every serial clock.  LinkDmg07_Poll() must therefore be
// called frequently enough to re-arm the serial port between adapter bytes;
// calling it only once per video frame is not sufficient.  Call
// LinkDmg07_TickFrame() exactly once per frame for timeout accounting.
//
// Transmission uses SIZE=1: every adapter packet contains four bytes, one for
// each physical player slot.  A console sends its local byte during byte 0 and
// sends zero during bytes 1..3, except while emitting an aligned FF restart
// packet.  The adapter broadcasts those four submitted bytes in the following
// packet.  The initial broadcast contains undefined adapter data and is
// discarded by this driver.

#define LINK_DMG07_PHASE_PING       ((u8)0)
#define LINK_DMG07_PHASE_REQUEST_AA ((u8)1)
#define LINK_DMG07_PHASE_WAIT_CC    ((u8)2)
#define LINK_DMG07_PHASE_TRANSFER   ((u8)3)

#define LINK_DMG07_MAX_PLAYERS  ((u8)4)
#define LINK_DMG07_PACKET_BYTES ((u8)4)
#define LINK_DMG07_SIZE         ((u8)1)
// Physical slots here are one-based; do not substitute the zero-based Link4 cooperative slot identifiers.
#define LINK_DMG07_NO_SLOT      ((u8)0)

// RATE $10 gives the shortest documented packet period for SIZE=1 while
// retaining the adapter's normal inter-byte pacing.  RATE $00 is not used.
#define LINK_DMG07_DEFAULT_RATE ((u8)0x10)

#ifndef LINK_DMG07_TIMEOUT_FRAMES
#define LINK_DMG07_TIMEOUT_FRAMES ((u8)12)
#endif

#define LINK_DMG07_ERR_NONE        ((u8)0)
#define LINK_DMG07_ERR_STATE       ((u8)1)
#define LINK_DMG07_ERR_NOT_PLAYER1 ((u8)2)
#define LINK_DMG07_ERR_NOT_READY   ((u8)3)
#define LINK_DMG07_ERR_TIMEOUT     ((u8)4)
#define LINK_DMG07_ERR_PROTOCOL    ((u8)5)
#define LINK_DMG07_ERR_OVERFLOW    ((u8)6)

#define LINK_DMG07_RESTART_NONE    ((u8)0)
#define LINK_DMG07_RESTART_PENDING ((u8)1)
#define LINK_DMG07_RESTART_SENDING ((u8)2)

// Initializes polling-only external-clock operation.  A zero RATE is replaced
// with LINK_DMG07_DEFAULT_RATE.  This driver disables only the serial IRQ bit.
void __stackcall LinkDmg07_Init(u8 rate);

// Services at most one completed serial byte and immediately re-arms SC=$80.
// Call continuously from the main loop, including between VBlanks.
void LinkDmg07_Poll();

// Advances saturating frame-based silence and handshake timeout counters.
void LinkDmg07_TickFrame();

// Return the discovery, start-handshake or transfer phase without polling.
u8 LinkDmg07_GetPhase();
// Return the one-based player slot, or LINK_DMG07_NO_SLOT before discovery.
u8 LinkDmg07_GetLocalSlot();

// Normalized mask: bit 0 is player 1, ..., bit 3 is player 4.
u8 LinkDmg07_GetConnectedMask();
// Return the last validated raw adapter status byte.
u8 LinkDmg07_GetLastStatus();
// Read the completed-discovery status flag without clearing it.
u8 LinkDmg07_HasPingStatus();
// Clear and report the completed-discovery flag; return zero if no new status exists.
u8 LinkDmg07_ConsumePingStatus();

// Only physical player 1 may request AA AA AA AA.  The bytes are aligned to
// the next FE/status/status/status ping packet.  All consoles enter transfer
// mode only after receiving four consecutive CC bytes.
u8 LinkDmg07_RequestTransmission();

// Any physical slot may schedule FF FF FF FF at the next four-byte packet
// boundary while remaining in TRANSFER phase.  The driver keeps sending aligned
// FF bytes until it sees the adapter's complete all-FF indicator packet, then
// returns to PING.  A transfer-phase silence timeout schedules the same recovery
// automatically and preserves the current byte position for clocks that resume.
u8 LinkDmg07_RequestRestart();
// Return whether aligned recovery is absent, pending or sending control traffic.
u8 LinkDmg07_GetRestartState();

// Sets the byte sampled when byte 0 of the next adapter packet is armed.
// Set it before the current packet completes (and before requesting start for
// the first submission).  During normal transfer, bytes 1..3 are sent as zero.
void __stackcall LinkDmg07_SetLocalByte(u8 value);

// The first incoming adapter data packet is discarded.  Thereafter ReadPacket
// copies four bytes in physical slot order (players 1..4).
u8 LinkDmg07_IsPipelinePrimed();
// Inspect the one-packet receive mailbox without consuming it.
u8 LinkDmg07_HasPacket();
// Consume the latest four-byte packet. A nonnull destination must hold four
// bytes; null discards the packet. Packet storage remains after the flag clears.
u8 __stackcall LinkDmg07_ReadPacket(u8 *dst);
// Read one cached player byte using a one-based slot. Invalid slots return
// zero; the function neither requires readiness nor consumes the packet.
u8 __stackcall LinkDmg07_GetPacketSlot(u8 player_slot);

// SentSequence increments when this console's packet byte is clocked out.
// PacketSequence labels the returned four-byte packet.  For a valid packet it
// is one less than SentSequence, making the DMG-07 one-packet delay explicit.
u8 LinkDmg07_GetSentSequence();
// Return the modulo-256 sequence associated with the most recently published packet.
u8 LinkDmg07_GetPacketSequence();

// SilenceFrames is reset by every completed serial byte.  TimeoutCount counts
// distinct serial-silence or AA/CC/restart handshake timeouts. DisconnectCount counts
// adapter-silence outages and ping-status membership drops.  All saturate.
u8 LinkDmg07_GetSilenceFrames();
// Return the saturating lifetime timeout count, preserved across protocol restarts.
u8 LinkDmg07_GetTimeoutCount();
// Return the saturating count of removed-player and previously-seen-adapter timeout events.
u8 LinkDmg07_GetDisconnectCount();
// Return the saturating protocol/overflow error count. Timeout and request
// validation paths use separate counters or latches and do not increment this value.
u8 LinkDmg07_GetErrorCount();
// Return the latest error latch without clearing counters or restarting the protocol.
u8 LinkDmg07_LastError();
// Clear only the latest error code, preserving counters and protocol state.
void LinkDmg07_ClearError();
