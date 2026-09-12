#pragma bank 1

#include "link.h"

u8 __critical_enter();
void __critical_leave(u8 token);
void __wait_vblank();
u8 LinkHw_ReadSB();
u8 LinkHw_ReadSC();
u8 LinkHw_ReadIF();
u8 LinkHw_ReadIE();
void LinkHw_WriteSB(u8 value);
void LinkHw_WriteSC(u8 value);
void LinkHw_WriteIF(u8 value);
void LinkHw_WriteIE(u8 value);

#define LINK_SC_START    ((u8)0x80)
#define LINK_SC_FAST     ((u8)0x02)
#define LINK_SC_INTERNAL ((u8)0x01)
#define LINK_IF_SERIAL   ((u8)0x08)
#define LINK_IE_SERIAL   ((u8)0x08)

#define LINK_PKT_STATE_IDLE ((u8)0)

__hram u8 Link_Mode;
__hram u8 Link_UseInterrupt;
__hram u8 Link_FastClock;
__hram u8 Link_Busy;
__hram u8 Link_LastRx;
__hram u8 Link_LastTx;
__hram u8 Link_RxReady;
__hram u8 Link_LastErrorCode;
__hram u8 Link_PacketState;
__hram u8 Link_TxSeq;
__hram u8 Link_RxSeq;
__hram u8 Link4_ModeState;
__hram u8 Link4_LocalSlot;
__hram u8 Link4_SlotCount;
__hram u8 Link4_SelectedPeer;
__hram u8 Link4_LastRxPeer;
__hram u8 Link4_LastTxPeer;
__hram u8 Link4_LastPacketPeer;

__wram u8 Link_RxQueue[LINK_RX_QUEUE_SIZE];
__wram u8 Link_RxHead;
__wram u8 Link_RxTail;

__wram u8 Link_TxQueue[LINK_TX_QUEUE_SIZE];
__wram u8 Link_TxHead;
__wram u8 Link_TxTail;

__wram u8 Link_CurrentPacketCmd;
__wram u8 Link_CurrentPacketLen;
__wram u8 Link_CurrentPacketBuf[LINK_PKT_MAX_PAYLOAD];
__wram u8 Link_CurrentPacketChecksum;
__wram u8 Link_PendingAck;

__wram u8 Link_ReceivedPacketReady;
__wram u8 Link_ReceivedPacketCmd;
__wram u8 Link_ReceivedPacketLen;
__wram u8 Link_ReceivedPacketBuf[LINK_PKT_MAX_PAYLOAD];
__wram u8 Link_ReceivedPacketChecksum;
__wram u8 Link_PacketRetryCount;
__wram u8 Link_PacketTimer;
__wram u8 Link4_RxQueue[LINK4_MAX_PLAYERS * LINK_RX_QUEUE_SIZE];
__wram u8 Link4_RxHead[LINK4_MAX_PLAYERS];
__wram u8 Link4_RxTail[LINK4_MAX_PLAYERS];
__wram u8 Link4_ReceivedPacketReady[LINK4_MAX_PLAYERS];
__wram u8 Link4_ReceivedPacketCmd[LINK4_MAX_PLAYERS];
__wram u8 Link4_ReceivedPacketLen[LINK4_MAX_PLAYERS];
__wram u8 Link4_ReceivedPacketBuf[LINK4_MAX_PLAYERS * LINK_PKT_MAX_PAYLOAD];

u8 Link_NextIndex(u8 index, u8 size) {
    index++;
    if (index >= size) return 0;
    return index;
}

u8 Link4_QueueOffset(u8 peer_slot, u8 index) {
    return (u8)((u8)(peer_slot * LINK_RX_QUEUE_SIZE) + index);
}

u8 Link4_PacketOffset(u8 peer_slot, u8 index) {
    return (u8)((u8)(peer_slot * LINK_PKT_MAX_PAYLOAD) + index);
}

u8 Link4_IsEnabledInternal() {
    return (u8)(Link4_ModeState != LINK4_MODE_NONE);
}

u8 Link4_IsValidRemoteSlotInternal(u8 peer_slot) {
    if (Link4_IsEnabledInternal() == 0) return 0;
    if (peer_slot >= Link4_SlotCount) return 0;
    if (peer_slot == Link4_LocalSlot) return 0;
    return 1;
}

u8 Link4_CurrentTransferPeerInternal() {
    if (Link4_ModeState == LINK4_MODE_HOST) return Link4_SelectedPeer;
    if (Link4_ModeState == LINK4_MODE_PEER) return LINK4_HOST_SLOT;
    return LINK4_INVALID_SLOT;
}

void Link_SetErrorInternal(u8 code) {
    if (code == LINK_ERR_NONE) return;
    Link_LastErrorCode = code;
}

void Link_SetSerialInterruptEnabled(u8 on) {
    u8 tok = __critical_enter();

    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));
    if (on != 0) LinkHw_WriteIE((u8)(LinkHw_ReadIE() | LINK_IE_SERIAL));
    else LinkHw_WriteIE((u8)(LinkHw_ReadIE() & (u8)~LINK_IE_SERIAL));

    __critical_leave(tok);
}

void Link_PushRxByte(u8 value) {
    u8 next;
    u8 tok = __critical_enter();

    next = Link_NextIndex(Link_RxHead, LINK_RX_QUEUE_SIZE);
    if (next == Link_RxTail) {
        __critical_leave(tok);
        Link_SetErrorInternal(LINK_ERR_OVERFLOW);
        return;
    }

    Link_RxQueue[(__safe_index u8)Link_RxHead] = value;
    Link_RxHead = next;
    Link_RxReady = 1;

    __critical_leave(tok);
}

void Link4_PushRxByte(u8 peer_slot, u8 value) {
    u8 next;
    u8 offset;
    u8 tok;

    if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) return;

    tok = __critical_enter();
    next = Link_NextIndex(Link4_RxHead[(__safe_index u8)peer_slot], LINK_RX_QUEUE_SIZE);
    if (next == Link4_RxTail[(__safe_index u8)peer_slot]) {
        __critical_leave(tok);
        Link_SetErrorInternal(LINK_ERR_OVERFLOW);
        return;
    }

    offset = Link4_QueueOffset(peer_slot, Link4_RxHead[(__safe_index u8)peer_slot]);
    Link4_RxQueue[(__safe_index u8)offset] = value;
    Link4_RxHead[(__safe_index u8)peer_slot] = next;

    __critical_leave(tok);
}

u8 Link_PopRxByte(u8 *out) {
    u8 value;
    u8 next;
    u8 tok = __critical_enter();

    if (Link_RxHead == Link_RxTail) {
        Link_RxReady = 0;
        __critical_leave(tok);
        return 0;
    }

    value = Link_RxQueue[(__safe_index u8)Link_RxTail];
    next = Link_NextIndex(Link_RxTail, LINK_RX_QUEUE_SIZE);
    Link_RxTail = next;
    if (Link_RxHead == Link_RxTail) Link_RxReady = 0;
    else Link_RxReady = 1;

    __critical_leave(tok);

    if (out != 0) *out = value;
    return 1;
}

u8 Link4_PopRxByte(u8 peer_slot, u8 *out) {
    u8 value;
    u8 next;
    u8 offset;
    u8 tok;

    if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) return 0;

    tok = __critical_enter();
    if (Link4_RxHead[(__safe_index u8)peer_slot] == Link4_RxTail[(__safe_index u8)peer_slot]) {
        __critical_leave(tok);
        return 0;
    }

    offset = Link4_QueueOffset(peer_slot, Link4_RxTail[(__safe_index u8)peer_slot]);
    value = Link4_RxQueue[(__safe_index u8)offset];
    next = Link_NextIndex(Link4_RxTail[(__safe_index u8)peer_slot], LINK_RX_QUEUE_SIZE);
    Link4_RxTail[(__safe_index u8)peer_slot] = next;

    __critical_leave(tok);

    if (out != 0) *out = value;
    return 1;
}

void Link_FinishTransfer() {
    u8 value = LinkHw_ReadSB();
    u8 peer_slot = Link4_CurrentTransferPeerInternal();

    Link_Busy = 0;
    Link_LastRx = value;
    Link4_LastRxPeer = peer_slot;
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));
    Link_PushRxByte(value);
    Link4_PushRxByte(peer_slot, value);
}

void Link_InitCommon() {
    u8 i;

    Link_Mode = LINK_MODE_SLAVE;
    Link_UseInterrupt = 0;
    Link_FastClock = LINK_CLOCK_NORMAL;
    Link_Busy = 0;
    Link_LastRx = 0xFF;
    Link_LastTx = 0xFF;
    Link_RxReady = 0;
    Link_LastErrorCode = LINK_ERR_NONE;
    Link_PacketState = LINK_PKT_STATE_IDLE;
    Link_TxSeq = 0;
    Link_RxSeq = 0;
    Link4_ModeState = LINK4_MODE_NONE;
    Link4_LocalSlot = LINK4_HOST_SLOT;
    Link4_SlotCount = 0;
    Link4_SelectedPeer = LINK4_INVALID_SLOT;
    Link4_LastRxPeer = LINK4_INVALID_SLOT;
    Link4_LastTxPeer = LINK4_INVALID_SLOT;
    Link4_LastPacketPeer = LINK4_INVALID_SLOT;

    Link_RxHead = 0;
    Link_RxTail = 0;
    Link_TxHead = 0;
    Link_TxTail = 0;

    Link_CurrentPacketCmd = 0;
    Link_CurrentPacketLen = 0;
    Link_CurrentPacketChecksum = 0;
    Link_PendingAck = 0;

    Link_ReceivedPacketReady = 0;
    Link_ReceivedPacketCmd = 0;
    Link_ReceivedPacketLen = 0;
    Link_ReceivedPacketChecksum = 0;
    Link_PacketRetryCount = 0;
    Link_PacketTimer = 0;

    for (i = 0; i < LINK_RX_QUEUE_SIZE; ++i) Link_RxQueue[(__safe_index u8)i] = 0;
    for (i = 0; i < LINK_TX_QUEUE_SIZE; ++i) Link_TxQueue[(__safe_index u8)i] = 0;
    for (i = 0; i < LINK4_MAX_PLAYERS; ++i) {
        Link4_RxHead[(__safe_index u8)i] = 0;
        Link4_RxTail[(__safe_index u8)i] = 0;
        Link4_ReceivedPacketReady[(__safe_index u8)i] = 0;
        Link4_ReceivedPacketCmd[(__safe_index u8)i] = 0;
        Link4_ReceivedPacketLen[(__safe_index u8)i] = 0;
    }
    for (i = 0; i < (u8)(LINK4_MAX_PLAYERS * LINK_RX_QUEUE_SIZE); ++i) {
        Link4_RxQueue[(__safe_index u8)i] = 0;
    }
    for (i = 0; i < LINK_PKT_MAX_PAYLOAD; ++i) {
        Link_CurrentPacketBuf[(__safe_index u8)i] = 0;
        Link_ReceivedPacketBuf[(__safe_index u8)i] = 0;
    }
    for (i = 0; i < (u8)(LINK4_MAX_PLAYERS * LINK_PKT_MAX_PAYLOAD); ++i) {
        Link4_ReceivedPacketBuf[(__safe_index u8)i] = 0;
    }

    LinkHw_WriteSB(0xFF);
    LinkHw_WriteSC(0x00);
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));
    Link_SetSerialInterruptEnabled(0);
}

void Link_InitMaster() {
    Link_InitCommon();
    Link_Mode = LINK_MODE_MASTER;
}

void Link_InitSlave() {
    Link_InitCommon();
    Link_Mode = LINK_MODE_SLAVE;
}

void __stackcall Link_SetUseInterrupt(u8 on) {
    if (on != 0) {
        Link_UseInterrupt = 1;
        Link_SetSerialInterruptEnabled(1);
    } else {
        Link_UseInterrupt = 0;
        Link_SetSerialInterruptEnabled(0);
    }
}

void __stackcall Link_SetFastClock(u8 on) {
    if (on != 0) Link_FastClock = LINK_CLOCK_FAST;
    else Link_FastClock = LINK_CLOCK_NORMAL;
}

u8 Link_IsBusy() {
    Link_Poll();
    return Link_Busy;
}

u8 Link_HasByte() {
    return Link_RxReady;
}

u8 Link_LastError() {
    return Link_LastErrorCode;
}

void Link_ClearError() {
    Link_LastErrorCode = LINK_ERR_NONE;
}

u8 __stackcall Link_BeginTransfer(u8 out) {
    u8 sc = LINK_SC_START;

    if (Link_Busy != 0) {
        Link_SetErrorInternal(LINK_ERR_BUSY);
        return LINK_ERR_BUSY;
    }

    LinkHw_WriteSB(out);
    Link_LastTx = out;
    Link4_LastTxPeer = Link4_CurrentTransferPeerInternal();
    Link_Busy = 1;
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));

    if (Link_Mode == LINK_MODE_MASTER) {
        sc = (u8)(sc | LINK_SC_INTERNAL);
        if (Link_FastClock != 0) sc = (u8)(sc | LINK_SC_FAST);
    }

    LinkHw_WriteSC(sc);
    return LINK_ERR_NONE;
}

#pragma bank 1

void Link_Poll() {
    if (Link_Busy == 0) return;
    if ((LinkHw_ReadSC() & LINK_SC_START) != 0) return;
    Link_FinishTransfer();
}

void Link_Cancel() {
    u8 tok = __critical_enter();

    Link_Busy = 0;
    LinkHw_WriteSC(0x00);
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));

    __critical_leave(tok);
}

#pragma bank 1

void Link_OnSerialIRQ() {
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & (u8)~LINK_IF_SERIAL));
    if (Link_Busy == 0) return;
    Link_FinishTransfer();
}

u8 __stackcall Link_TryReadByte(u8 *out) {
    return Link_PopRxByte(out);
}

// Blocking/convenience readers are not part of the per-frame serial hot path.
// Keep them banked with Link4 so the fixed bank has deterministic headroom.
#pragma bank 1

u8 Link_ReadByte() {
    u8 value = 0;
    if (Link_PopRxByte(&value) != 0) return value;
    return 0;
}

u8 __stackcall Link_WaitByte(u16 timeout_frames, u8 *out) {
    u16 waited = 0;

    if (out == 0) return 0;

    Link_Poll();
    if (Link_TryReadByte(out) != 0) return 1;

    while (waited < timeout_frames) {
        __wait_vblank();
        waited++;
        Link_Poll();
        if (Link_TryReadByte(out) != 0) return 1;
    }

    Link_SetErrorInternal(LINK_ERR_TIMEOUT);
    return 0;
}

// Cooperative Link4 helpers are banked so two-player applications can keep
// the timing-critical raw serial path in the fixed bank.

u8 Link4_ValidateConfig(u8 mode, u8 local_slot, u8 slot_count) {
    if (slot_count < 2 || slot_count > LINK4_MAX_PLAYERS) return LINK_ERR_PROTOCOL;
    if (mode == LINK4_MODE_HOST) {
        if (local_slot != LINK4_HOST_SLOT) return LINK_ERR_PROTOCOL;
        return LINK_ERR_NONE;
    }
    if (mode == LINK4_MODE_PEER) {
        if (local_slot == LINK4_HOST_SLOT) return LINK_ERR_PROTOCOL;
        if (local_slot >= slot_count) return LINK_ERR_PROTOCOL;
        return LINK_ERR_NONE;
    }
    return LINK_ERR_PROTOCOL;
}

void Link4_ApplyConfig(u8 mode, u8 local_slot, u8 slot_count) {
    Link4_ModeState = mode;
    Link4_LocalSlot = local_slot;
    Link4_SlotCount = slot_count;
    Link4_LastRxPeer = LINK4_INVALID_SLOT;
    Link4_LastTxPeer = LINK4_INVALID_SLOT;
    Link4_LastPacketPeer = LINK4_INVALID_SLOT;
    if (mode == LINK4_MODE_HOST) Link4_SelectedPeer = 1;
    else if (mode == LINK4_MODE_PEER) Link4_SelectedPeer = LINK4_HOST_SLOT;
    else Link4_SelectedPeer = LINK4_INVALID_SLOT;
}

u8 __stackcall Link4_InitHost(u8 slot_count) {
    u8 err;

    Link_InitMaster();
    err = Link4_ValidateConfig(LINK4_MODE_HOST, LINK4_HOST_SLOT, slot_count);
    if (err != LINK_ERR_NONE) {
        Link_SetErrorInternal(err);
        return err;
    }

    Link4_ApplyConfig(LINK4_MODE_HOST, LINK4_HOST_SLOT, slot_count);
    return LINK_ERR_NONE;
}

u8 __stackcall Link4_InitPeer(u8 local_slot, u8 slot_count) {
    u8 err;

    Link_InitSlave();
    err = Link4_ValidateConfig(LINK4_MODE_PEER, local_slot, slot_count);
    if (err != LINK_ERR_NONE) {
        Link_SetErrorInternal(err);
        return err;
    }

    Link4_ApplyConfig(LINK4_MODE_PEER, local_slot, slot_count);
    return LINK_ERR_NONE;
}

u8 Link4_GetMode() {
    return Link4_ModeState;
}

u8 Link4_GetLocalSlot() {
    return Link4_LocalSlot;
}

u8 Link4_GetSlotCount() {
    return Link4_SlotCount;
}

u8 Link4_GetSelectedPeer() {
    return Link4_SelectedPeer;
}

u8 Link4_GetLastRxPeer() {
    return Link4_LastRxPeer;
}

u8 Link4_GetLastTxPeer() {
    return Link4_LastTxPeer;
}

u8 __stackcall Link4_SelectPeer(u8 peer_slot) {
    Link_Poll();

    if (Link4_IsEnabledInternal() == 0) {
        Link_SetErrorInternal(LINK_ERR_PROTOCOL);
        return LINK_ERR_PROTOCOL;
    }

    if (Link_Busy != 0 || Link_PacketState != LINK_PKT_STATE_IDLE || Link_PendingAck != 0) {
        Link_SetErrorInternal(LINK_ERR_BUSY);
        return LINK_ERR_BUSY;
    }

    if (Link4_ModeState == LINK4_MODE_HOST) {
        if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) {
            Link_SetErrorInternal(LINK_ERR_PROTOCOL);
            return LINK_ERR_PROTOCOL;
        }
        Link4_SelectedPeer = peer_slot;
        return LINK_ERR_NONE;
    }

    if (Link4_ModeState == LINK4_MODE_PEER && peer_slot == LINK4_HOST_SLOT) {
        Link4_SelectedPeer = LINK4_HOST_SLOT;
        return LINK_ERR_NONE;
    }

    Link_SetErrorInternal(LINK_ERR_PROTOCOL);
    return LINK_ERR_PROTOCOL;
}

u8 __stackcall Link4_BeginTransferTo(u8 peer_slot, u8 out) {
    u8 err = Link4_SelectPeer(peer_slot);
    if (err != LINK_ERR_NONE) return err;
    return Link_BeginTransfer(out);
}

u8 __stackcall Link4_HasByteFrom(u8 peer_slot) {
    if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) return 0;
    if (Link4_RxHead[(__safe_index u8)peer_slot] == Link4_RxTail[(__safe_index u8)peer_slot]) return 0;
    return 1;
}

u8 __stackcall Link4_TryReadByteFrom(u8 peer_slot, u8 *out) {
    return Link4_PopRxByte(peer_slot, out);
}

u8 __stackcall Link4_TryReadByteAny(u8 *peer_slot, u8 *out) {
    u8 slot;

    if (Link4_IsEnabledInternal() == 0) return 0;

    for (slot = 0; slot < Link4_SlotCount; ++slot) {
        if (slot != Link4_LocalSlot) {
            if (Link4_PopRxByte(slot, out) != 0) {
                if (peer_slot != 0) *peer_slot = slot;
                return 1;
            }
        }
    }

    return 0;
}
