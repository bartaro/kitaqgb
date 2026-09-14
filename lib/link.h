#pragma once

// Shared Game Boy serial link library for KITAQGB projects.
//
// Integration notes:
// - Compile a hardware-register file before link.c / link_packet.c so SB, SC,
//   IF_REG, and IE_REG are already declared.
// - Call Link_InitMaster() on the clock-source side and Link_InitSlave() on the
//   other side.
// - Call Link_Poll() or Link_PollPacket() regularly from the main loop.
// - If you enable serial IRQ mode, call Link_OnSerialIRQ() from your own
//   serial interrupt handler at vector 0x0058.
// - The packet layer is intentionally small and one-deep. It works best as a
//   master-driven command/response layer. The low-level byte API remains the
//   escape hatch for custom protocols.

#define LINK_MODE_SLAVE   ((u8)0)
#define LINK_MODE_MASTER  ((u8)1)

#define LINK_CLOCK_NORMAL ((u8)0)
#define LINK_CLOCK_FAST   ((u8)1)

#define LINK_ERR_NONE        ((u8)0)
#define LINK_ERR_BUSY        ((u8)1)
#define LINK_ERR_TIMEOUT     ((u8)2)
#define LINK_ERR_OVERFLOW    ((u8)3)
#define LINK_ERR_PROTOCOL    ((u8)4)
#define LINK_ERR_DISCONNECT  ((u8)5)

#define LINK_PKT_MAX_PAYLOAD ((u8)24)
#define LINK_RX_QUEUE_SIZE   ((u8)8)
#define LINK_TX_QUEUE_SIZE   ((u8)8)

#define LINK4_MODE_NONE      ((u8)0)
#define LINK4_MODE_HOST      ((u8)1)
#define LINK4_MODE_PEER      ((u8)2)
#define LINK4_MAX_PLAYERS    ((u8)4)
#define LINK4_HOST_SLOT      ((u8)0)
#define LINK4_INVALID_SLOT   ((u8)0xFF)

// Reset byte queues, packet mailboxes, cooperative-peer state and errors, then
// disable serial transfers and serial IRQ delivery. Existing data is discarded.
void Link_InitCommon();
// Reset link state and select the internal-clock role for subsequent transfers.
void Link_InitMaster();
// Reset link state and select the external-clock role for subsequent transfers.
void Link_InitSlave();
// Enable or disable serial IRQ delivery and clear its pending flag. Enabling
// does not install the handler or globally enable interrupts.
void __stackcall Link_SetUseInterrupt(u8 on);
// Select the fast-clock bit for future master transfers; this does not restart
// an active transfer or verify that the hardware supports CGB fast mode.
void __stackcall Link_SetFastClock(u8 on);

// Poll once before returning busy state, so this query may collect a completed byte.
u8 Link_IsBusy();
// Read the ordinary receive-ready flag without polling hardware.
u8 Link_HasByte();
// Return the latest latched error without clearing it.
u8 Link_LastError();
// Clear the error latch without resetting queues or an active transfer.
void Link_ClearError();

// Start one full-duplex byte exchange, or return BUSY without replacing an
// active transfer. Slave mode arms reception and waits for external clocks.
u8 __stackcall Link_BeginTransfer(u8 out);
// Collect a pending transfer only after hardware clears the SC start bit.
void Link_Poll();
// Stop the hardware transfer under interrupt protection. Queued receive bytes
// and packet-layer state remain available; this is not a full protocol reset.
void Link_Cancel();
// Acknowledge serial IRQ and finish a software-busy transfer. Call this from
// the actual serial interrupt handler, not an unrelated interrupt.
void Link_OnSerialIRQ();

// Pop the ordinary receive queue without waiting or polling hardware; null discards a byte.
u8 __stackcall Link_TryReadByte(u8 *out);
// Pop one byte, returning zero when empty. Use Link_TryReadByte when a received
// zero byte must be distinguished from an empty queue.
u8 Link_ReadByte();
// Require an output pointer, poll immediately, then wait at most timeout_frames
// VBlanks for a byte. Zero timeout still performs the initial poll and read.
u8 __stackcall Link_WaitByte(u16 timeout_frames, u8 *out);

// Validate payload size and idle packet state, copy payload bytes into owned
// storage and schedule transmission. Return success means queued, not acknowledged;
// keep polling to progress the exchange and inspect the error latch.
u8 __stackcall Link_SendPacket(const u8 *data, u8 len, u8 cmd);
// Drain received bytes, advance one outgoing exchange and update the retry
// timer. Timeout ticks count calls without progress, not hardware frames; call
// with a controlled cadence and avoid mixing raw reads with packet consumption.
void Link_PollPacket();
// Inspect the shared completed-packet flag without advancing the protocol.
u8 Link_HasPacket();
// Consume the shared packet mailbox and its matching peer flag. Any output
// may be null; a nonnull destination needs room for the received payload because
// there is no destination-capacity argument.
u8 __stackcall Link_ReadPacket(u8 *cmd, u8 *len, u8 *dst);

// Cooperative 4-player adapter helpers.
//
// This layer models the common host-selected adapter pattern:
// - Slot 0 is always the host.
// - The host selects one active peer slot at a time.
// - Packet mailboxes stay one-deep per peer.
// - The original Link_* API remains available for normal 2-player projects.
// Reset as master, validate slot count and select logical peer one. Invalid configuration still discards old link state.
u8 __stackcall Link4_InitHost(u8 slot_count);
// Reset as slave and validate the local nonzero slot and total slot count.
// An invalid configuration still discards the previous link state.
u8 __stackcall Link4_InitPeer(u8 local_slot, u8 slot_count);
// Return the configured cooperative role; this is not a hardware connection probe.
u8 Link4_GetMode();
// Return the configured zero-based local slot.
u8 Link4_GetLocalSlot();
// Return the configured number of logical slots, not a live connection count.
u8 Link4_GetSlotCount();
// Return the current logical destination, or the invalid-slot sentinel when unset.
u8 Link4_GetSelectedPeer();
// Return the peer attributed to the last completed byte transfer.
u8 Link4_GetLastRxPeer();
// Return the peer attributed when the last byte transfer was started.
u8 Link4_GetLastTxPeer();
// Poll completion, then change the logical destination only while byte and
// packet state are idle. A peer can select only the host. No adapter-routing
// command is sent here; the transport must provide the matching physical route.
u8 __stackcall Link4_SelectPeer(u8 peer_slot);
// Select the logical peer and start a byte exchange, returning the first error.
u8 __stackcall Link4_BeginTransferTo(u8 peer_slot, u8 out);
// Inspect a valid peer ring without polling the hardware.
u8 __stackcall Link4_HasByteFrom(u8 peer_slot);
// Pop one byte from the selected peer ring; null output discards it.
u8 __stackcall Link4_TryReadByteFrom(u8 peer_slot, u8 *out);
// Scan remote rings in increasing slot order and return the first available
// byte. Null output pointers are allowed; lower slots have polling priority.
u8 __stackcall Link4_TryReadByteAny(u8 *peer_slot, u8 *out);
// Select a logical peer, then copy and schedule a packet; return the first error.
u8 __stackcall Link4_SendPacketTo(u8 peer_slot, const u8 *data, u8 len, u8 cmd);
// Advance the shared packet engine; this does not poll every logical peer in turn.
void Link4_PollPacket();
// Inspect the one-packet mailbox for a valid remote slot without consuming it.
u8 __stackcall Link4_HasPacketFrom(u8 peer_slot);
// Consume a peer mailbox, copying to optional outputs. A nonnull destination
// needs room for the entire payload. Clear the shared mailbox only when it
// refers to this same peer.
u8 __stackcall Link4_ReadPacketFrom(u8 peer_slot, u8 *cmd, u8 *len, u8 *dst);
