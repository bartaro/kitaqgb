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

void Link_InitCommon();
void Link_InitMaster();
void Link_InitSlave();
void __stackcall Link_SetUseInterrupt(u8 on);
void __stackcall Link_SetFastClock(u8 on);

u8 Link_IsBusy();
u8 Link_HasByte();
u8 Link_LastError();
void Link_ClearError();

u8 __stackcall Link_BeginTransfer(u8 out);
void Link_Poll();
void Link_Cancel();
void Link_OnSerialIRQ();

u8 __stackcall Link_TryReadByte(u8 *out);
u8 Link_ReadByte();
u8 __stackcall Link_WaitByte(u16 timeout_frames, u8 *out);

u8 __stackcall Link_SendPacket(const u8 *data, u8 len, u8 cmd);
void Link_PollPacket();
u8 Link_HasPacket();
u8 __stackcall Link_ReadPacket(u8 *cmd, u8 *len, u8 *dst);

// Cooperative 4-player adapter helpers.
//
// This layer models the common host-selected adapter pattern:
// - Slot 0 is always the host.
// - The host selects one active peer slot at a time.
// - Packet mailboxes stay one-deep per peer.
// - The original Link_* API remains available for normal 2-player projects.
u8 __stackcall Link4_InitHost(u8 slot_count);
u8 __stackcall Link4_InitPeer(u8 local_slot, u8 slot_count);
u8 Link4_GetMode();
u8 Link4_GetLocalSlot();
u8 Link4_GetSlotCount();
u8 Link4_GetSelectedPeer();
u8 Link4_GetLastRxPeer();
u8 Link4_GetLastTxPeer();
u8 __stackcall Link4_SelectPeer(u8 peer_slot);
u8 __stackcall Link4_BeginTransferTo(u8 peer_slot, u8 out);
u8 __stackcall Link4_HasByteFrom(u8 peer_slot);
u8 __stackcall Link4_TryReadByteFrom(u8 peer_slot, u8 *out);
u8 __stackcall Link4_TryReadByteAny(u8 *peer_slot, u8 *out);
u8 __stackcall Link4_SendPacketTo(u8 peer_slot, const u8 *data, u8 len, u8 cmd);
void Link4_PollPacket();
u8 __stackcall Link4_HasPacketFrom(u8 peer_slot);
u8 __stackcall Link4_ReadPacketFrom(u8 peer_slot, u8 *cmd, u8 *len, u8 *dst);
