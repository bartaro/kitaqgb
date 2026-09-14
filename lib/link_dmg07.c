#pragma bank 0

#include "link_dmg07.h"

u8 LinkHw_ReadSB();
u8 LinkHw_ReadSC();
u8 LinkHw_ReadIF();
u8 LinkHw_ReadIE();
void LinkHw_WriteSB(u8 value);
void LinkHw_WriteSC(u8 value);
void LinkHw_WriteIF(u8 value);
void LinkHw_WriteIE(u8 value);

#define LINK_DMG07_SC_EXTERNAL_START ((u8)0x80)
#define LINK_DMG07_IRQ_SERIAL_CLEAR  ((u8)0xF7)
#define LINK_DMG07_PING_HEADER       ((u8)0xFE)
#define LINK_DMG07_ACK               ((u8)0x88)
#define LINK_DMG07_START             ((u8)0xAA)
#define LINK_DMG07_START_REPLY       ((u8)0xCC)

__wram u8 LinkDmg07_Phase;
__wram u8 LinkDmg07_Rate;
__wram u8 LinkDmg07_PingIndex;
__wram u8 LinkDmg07_LocalSlot;
__wram u8 LinkDmg07_ConnectedMask;
__wram u8 LinkDmg07_LastStatus;
__wram u8 LinkDmg07_PingOldMask;
__wram u8 LinkDmg07_PingStatusReady;
__wram u8 LinkDmg07_StartRequested;
__wram u8 LinkDmg07_StartBytesArmed;
__wram u8 LinkDmg07_CcCount;
__wram u8 LinkDmg07_DataIndex;
__wram u8 LinkDmg07_LocalByte;
__wram u8 LinkDmg07_RxBuild[LINK_DMG07_PACKET_BYTES];
__wram u8 LinkDmg07_RxPacket[LINK_DMG07_PACKET_BYTES];
__wram u8 LinkDmg07_PacketReady;
__wram u8 LinkDmg07_PipelinePrimed;
__wram u8 LinkDmg07_SentSequence;
__wram u8 LinkDmg07_PacketSequence;
__wram u8 LinkDmg07_RestartState;
__wram u8 LinkDmg07_SilenceFrames;
__wram u8 LinkDmg07_HandshakeFrames;
__wram u8 LinkDmg07_TimeoutCount;
__wram u8 LinkDmg07_DisconnectCount;
__wram u8 LinkDmg07_ErrorCount;
__wram u8 LinkDmg07_LastErrorCode;
__wram u8 LinkDmg07_SeenAdapter;
__wram u8 LinkDmg07_TimeoutLatched;

// Increment a byte counter without wrapping past 0xFF.
u8 LinkDmg07_IncrementSaturatingInternal(u8 value) {
    if (value != 0xFF) value++;
    return value;
}

// Load the next response, clear pending serial IRQ and arm external-clock
// transfer. The adapter supplies clocks; this helper does not wait for completion.
void LinkDmg07_ArmInternal(u8 value) {
    LinkHw_WriteSB(value);
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & LINK_DMG07_IRQ_SERIAL_CLEAR));
    LinkHw_WriteSC(LINK_DMG07_SC_EXTERNAL_START);
}

// Record the latest error and increment the saturating runtime-error count.
void LinkDmg07_SetRuntimeErrorInternal(u8 code) {
    LinkDmg07_LastErrorCode = code;
    LinkDmg07_ErrorCount = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_ErrorCount);
}

// Discard transfer/handshake readiness and return to adapter discovery.
// Keep lifetime counters, the configured rate and the local application byte.
void LinkDmg07_ResetToPingInternal() {
    LinkDmg07_Phase = LINK_DMG07_PHASE_PING;
    LinkDmg07_PingIndex = 0;
    LinkDmg07_LocalSlot = LINK_DMG07_NO_SLOT;
    LinkDmg07_ConnectedMask = 0;
    LinkDmg07_LastStatus = 0;
    LinkDmg07_PingOldMask = 0;
    LinkDmg07_PingStatusReady = 0;
    LinkDmg07_StartRequested = 0;
    LinkDmg07_StartBytesArmed = 0;
    LinkDmg07_CcCount = 0;
    LinkDmg07_DataIndex = 0;
    LinkDmg07_PacketReady = 0;
    LinkDmg07_PipelinePrimed = 0;
    LinkDmg07_RestartState = LINK_DMG07_RESTART_NONE;
    LinkDmg07_HandshakeFrames = 0;
}

// Validate the one-based player slot and reserved bit before updating the
// connection mask. A changed local slot is rejected until discovery is reset.
void LinkDmg07_ApplyStatusInternal(u8 status) {
    u8 slot;

    slot = (u8)(status & (u8)0x07);
    if (((status & (u8)0x08) != 0) || (slot == 0) || (slot > LINK_DMG07_MAX_PLAYERS)) {
        LinkDmg07_SetRuntimeErrorInternal(LINK_DMG07_ERR_PROTOCOL);
        return;
    }

    if ((LinkDmg07_LocalSlot != LINK_DMG07_NO_SLOT) && (LinkDmg07_LocalSlot != slot)) {
        LinkDmg07_SetRuntimeErrorInternal(LINK_DMG07_ERR_PROTOCOL);
        return;
    }

    LinkDmg07_LocalSlot = slot;
    LinkDmg07_ConnectedMask = (u8)((status >> 4) & (u8)0x0F);
    LinkDmg07_LastStatus = status;
}

// Latch a complete status exchange and count a disconnect event when one
// or more previously connected players disappear from the mask.
void LinkDmg07_CompletePingInternal() {
    u8 removed;

    removed = (u8)(LinkDmg07_PingOldMask & (u8)(LinkDmg07_ConnectedMask ^ (u8)0x0F));
    if (removed != 0) {
        LinkDmg07_DisconnectCount = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_DisconnectCount);
    }
    LinkDmg07_PingStatusReady = 1;
}

// Respond to discovery with ACK, rate and packet size. A start reply enters
// the CC handshake; only player one can arm a requested AA start sequence.
u8 LinkDmg07_ProcessPingInternal(u8 value) {
    if (value == LINK_DMG07_START_REPLY) {
        LinkDmg07_Phase = LINK_DMG07_PHASE_WAIT_CC;
        LinkDmg07_CcCount = 1;
        LinkDmg07_HandshakeFrames = 0;
        LinkDmg07_PingIndex = 0;
        return 0;
    }

    if (LinkDmg07_PingIndex == 0) {
        if (value == LINK_DMG07_PING_HEADER) {
            LinkDmg07_PingOldMask = LinkDmg07_ConnectedMask;
            LinkDmg07_PingIndex = 1;
        }
        return LINK_DMG07_ACK;
    }

    if (value == LINK_DMG07_PING_HEADER) {
        LinkDmg07_SetRuntimeErrorInternal(LINK_DMG07_ERR_PROTOCOL);
        LinkDmg07_PingOldMask = LinkDmg07_ConnectedMask;
        LinkDmg07_PingIndex = 1;
        return LINK_DMG07_ACK;
    }

    LinkDmg07_ApplyStatusInternal(value);

    if (LinkDmg07_PingIndex == 1) {
        LinkDmg07_PingIndex = 2;
        return LinkDmg07_Rate;
    }

    if (LinkDmg07_PingIndex == 2) {
        LinkDmg07_PingIndex = 3;
        return LINK_DMG07_SIZE;
    }

    LinkDmg07_PingIndex = 0;
    LinkDmg07_CompletePingInternal();
    if ((LinkDmg07_StartRequested != 0) && (LinkDmg07_LocalSlot == 1)) {
        LinkDmg07_StartRequested = 0;
        LinkDmg07_Phase = LINK_DMG07_PHASE_REQUEST_AA;
        LinkDmg07_StartBytesArmed = 1;
        LinkDmg07_HandshakeFrames = 0;
        return LINK_DMG07_START;
    }
    return LINK_DMG07_ACK;
}

// Emit at most four AA start bytes, then wait for CC replies. A received CC
// can enter the wait phase earlier with one reply already counted.
u8 LinkDmg07_ProcessRequestInternal(u8 value) {
    if (value == LINK_DMG07_START_REPLY) {
        LinkDmg07_Phase = LINK_DMG07_PHASE_WAIT_CC;
        LinkDmg07_CcCount = 1;
        LinkDmg07_HandshakeFrames = 0;
        return 0;
    }

    if (LinkDmg07_StartBytesArmed < 4) {
        LinkDmg07_StartBytesArmed++;
        return LINK_DMG07_START;
    }

    LinkDmg07_Phase = LINK_DMG07_PHASE_WAIT_CC;
    LinkDmg07_CcCount = 0;
    LinkDmg07_HandshakeFrames = 0;
    return 0;
}

// Require four consecutive CC replies before entering transfer mode. Reset
// the receive pipeline and sequence counters at that transition.
u8 LinkDmg07_ProcessWaitCcInternal(u8 value) {
    if (value == LINK_DMG07_START_REPLY) {
        if (LinkDmg07_CcCount < 4) LinkDmg07_CcCount++;
    } else {
        LinkDmg07_CcCount = 0;
    }

    if (LinkDmg07_CcCount >= 4) {
        LinkDmg07_Phase = LINK_DMG07_PHASE_TRANSFER;
        LinkDmg07_DataIndex = 0;
        LinkDmg07_PipelinePrimed = 0;
        LinkDmg07_PacketReady = 0;
        LinkDmg07_SentSequence = 0;
        LinkDmg07_PacketSequence = 0;
        LinkDmg07_RestartState = LINK_DMG07_RESTART_NONE;
        LinkDmg07_HandshakeFrames = 0;
        return LinkDmg07_LocalByte;
    }
    return 0;
}

// Assemble four player bytes per packet and discard the first pipeline fill.
// Later packets overwrite the one-deep mailbox, recording overflow if unread.
// An all-FF packet restarts discovery; restart traffic is never delivered as data.
u8 LinkDmg07_ProcessTransferInternal(u8 value) {
    u8 copy_value;

    LinkDmg07_RxBuild[(__safe_index u8)LinkDmg07_DataIndex] = value;
    if ((LinkDmg07_DataIndex == 0) &&
        (LinkDmg07_RestartState != LINK_DMG07_RESTART_SENDING)) {
        LinkDmg07_SentSequence++;
    }

    if (LinkDmg07_DataIndex < 3) {
        LinkDmg07_DataIndex++;
        if (LinkDmg07_RestartState == LINK_DMG07_RESTART_SENDING) {
            return 0xFF;
        }
        return 0;
    }

    if ((LinkDmg07_RxBuild[(__safe_index u8)0] == 0xFF) &&
        (LinkDmg07_RxBuild[(__safe_index u8)1] == 0xFF) &&
        (LinkDmg07_RxBuild[(__safe_index u8)2] == 0xFF) &&
        (LinkDmg07_RxBuild[(__safe_index u8)3] == 0xFF)) {
        LinkDmg07_ResetToPingInternal();
        return LINK_DMG07_ACK;
    }

    if (LinkDmg07_RestartState == LINK_DMG07_RESTART_SENDING) {
        // Recovery traffic is control data, not an application packet.  Keep
        // packet alignment and continue until the adapter returns its all-FF
        // indicator packet.
        LinkDmg07_DataIndex = 0;
        return 0xFF;
    }

    if (LinkDmg07_PipelinePrimed == 0) {
        LinkDmg07_PipelinePrimed = 1;
    } else {
        if (LinkDmg07_PacketReady != 0) {
            LinkDmg07_SetRuntimeErrorInternal(LINK_DMG07_ERR_OVERFLOW);
        }
        copy_value = LinkDmg07_RxBuild[(__safe_index u8)0];
        LinkDmg07_RxPacket[(__safe_index u8)0] = copy_value;
        copy_value = LinkDmg07_RxBuild[(__safe_index u8)1];
        LinkDmg07_RxPacket[(__safe_index u8)1] = copy_value;
        copy_value = LinkDmg07_RxBuild[(__safe_index u8)2];
        LinkDmg07_RxPacket[(__safe_index u8)2] = copy_value;
        copy_value = LinkDmg07_RxBuild[(__safe_index u8)3];
        LinkDmg07_RxPacket[(__safe_index u8)3] = copy_value;
        LinkDmg07_PacketSequence = (u8)(LinkDmg07_SentSequence - 1);
        LinkDmg07_PacketReady = 1;
    }

    LinkDmg07_DataIndex = 0;
    if (LinkDmg07_RestartState == LINK_DMG07_RESTART_PENDING) {
        LinkDmg07_RestartState = LINK_DMG07_RESTART_SENDING;
        LinkDmg07_HandshakeFrames = 0;
        return 0xFF;
    }
    return LinkDmg07_LocalByte;
}

// Reset the polling driver and counters, substitute the default rate for zero,
// disable serial IRQ delivery and arm an external-clock discovery response.
void __stackcall LinkDmg07_Init(u8 rate) {
    u8 i;

    LinkDmg07_Rate = rate;
    if (LinkDmg07_Rate == 0) LinkDmg07_Rate = LINK_DMG07_DEFAULT_RATE;
    LinkDmg07_LocalByte = 0;
    LinkDmg07_SentSequence = 0;
    LinkDmg07_PacketSequence = 0;
    LinkDmg07_RestartState = LINK_DMG07_RESTART_NONE;
    LinkDmg07_SilenceFrames = 0;
    LinkDmg07_TimeoutCount = 0;
    LinkDmg07_DisconnectCount = 0;
    LinkDmg07_ErrorCount = 0;
    LinkDmg07_LastErrorCode = LINK_DMG07_ERR_NONE;
    LinkDmg07_SeenAdapter = 0;
    LinkDmg07_TimeoutLatched = 0;
    for (i = 0; i < LINK_DMG07_PACKET_BYTES; ++i) {
        LinkDmg07_RxBuild[(__safe_index u8)i] = 0;
        LinkDmg07_RxPacket[(__safe_index u8)i] = 0;
    }
    LinkDmg07_ResetToPingInternal();

    LinkHw_WriteIE((u8)(LinkHw_ReadIE() & LINK_DMG07_IRQ_SERIAL_CLEAR));
    LinkHw_WriteIF((u8)(LinkHw_ReadIF() & LINK_DMG07_IRQ_SERIAL_CLEAR));
    LinkDmg07_ArmInternal(LINK_DMG07_ACK);
}

// Process at most one completed serial byte, dispatch it to the active phase
// and immediately arm the next response. Poll frequently enough for adapter timing.
void LinkDmg07_Poll() {
    u8 value;
    u8 next;

    if ((LinkHw_ReadSC() & LINK_DMG07_SC_EXTERNAL_START) != 0) return;

    value = LinkHw_ReadSB();
    LinkDmg07_SilenceFrames = 0;
    LinkDmg07_TimeoutLatched = 0;
    LinkDmg07_SeenAdapter = 1;

    next = 0;
    if (LinkDmg07_Phase == LINK_DMG07_PHASE_PING) {
        next = LinkDmg07_ProcessPingInternal(value);
    } else if (LinkDmg07_Phase == LINK_DMG07_PHASE_REQUEST_AA) {
        next = LinkDmg07_ProcessRequestInternal(value);
    } else if (LinkDmg07_Phase == LINK_DMG07_PHASE_WAIT_CC) {
        next = LinkDmg07_ProcessWaitCcInternal(value);
    } else if (LinkDmg07_Phase == LINK_DMG07_PHASE_TRANSFER) {
        next = LinkDmg07_ProcessTransferInternal(value);
    } else {
        LinkDmg07_SetRuntimeErrorInternal(LINK_DMG07_ERR_PROTOCOL);
        LinkDmg07_ResetToPingInternal();
        next = LINK_DMG07_ACK;
    }

    LinkDmg07_ArmInternal(next);
}

// Count and latch a timeout. During transfer, preserve the armed byte and
// packet alignment until a boundary allows recovery; otherwise restart discovery.
void LinkDmg07_TimeoutInternal(u8 is_disconnect) {
    LinkDmg07_TimeoutCount = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_TimeoutCount);
    LinkDmg07_LastErrorCode = LINK_DMG07_ERR_TIMEOUT;
    if ((is_disconnect != 0) && (LinkDmg07_SeenAdapter != 0)) {
        LinkDmg07_DisconnectCount = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_DisconnectCount);
    }
    LinkDmg07_SeenAdapter = 0;
    LinkDmg07_TimeoutLatched = 1;

    if (LinkDmg07_Phase == LINK_DMG07_PHASE_TRANSFER) {
        // SC is already armed for the current external-clock byte.  Do not
        // rewrite SB or lose the four-byte position.  When clocks resume, the
        // current packet finishes and ProcessTransfer starts an aligned FF
        // restart packet at the following boundary.
        if (LinkDmg07_RestartState == LINK_DMG07_RESTART_NONE) {
            LinkDmg07_RestartState = LINK_DMG07_RESTART_PENDING;
        }
        LinkDmg07_HandshakeFrames = 0;
        return;
    }

    LinkDmg07_ResetToPingInternal();
    LinkDmg07_ArmInternal(LINK_DMG07_ACK);
}

// Advance silence and handshake watchdogs once per application frame. Serial
// polling resets silence; a timeout remains latched until another byte arrives.
void LinkDmg07_TickFrame() {
    LinkDmg07_SilenceFrames = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_SilenceFrames);

    if ((LinkDmg07_Phase == LINK_DMG07_PHASE_REQUEST_AA) ||
        (LinkDmg07_Phase == LINK_DMG07_PHASE_WAIT_CC) ||
        (LinkDmg07_RestartState == LINK_DMG07_RESTART_SENDING)) {
        LinkDmg07_HandshakeFrames = LinkDmg07_IncrementSaturatingInternal(LinkDmg07_HandshakeFrames);
    } else {
        LinkDmg07_HandshakeFrames = 0;
    }

    if ((LinkDmg07_SilenceFrames >= LINK_DMG07_TIMEOUT_FRAMES) &&
        (LinkDmg07_TimeoutLatched == 0)) {
        LinkDmg07_TimeoutInternal(1);
        return;
    }

    if ((LinkDmg07_HandshakeFrames >= LINK_DMG07_TIMEOUT_FRAMES) &&
        (LinkDmg07_TimeoutLatched == 0)) {
        LinkDmg07_TimeoutInternal(0);
    }
}

// Return the discovery, start-handshake or transfer phase without polling.
u8 LinkDmg07_GetPhase() {
    return LinkDmg07_Phase;
}

// Return the one-based player slot, or LINK_DMG07_NO_SLOT before discovery.
u8 LinkDmg07_GetLocalSlot() {
    return LinkDmg07_LocalSlot;
}

// Return the last accepted four-player connection mask; this is a cached status.
u8 LinkDmg07_GetConnectedMask() {
    return LinkDmg07_ConnectedMask;
}

// Return the last validated raw adapter status byte.
u8 LinkDmg07_GetLastStatus() {
    return LinkDmg07_LastStatus;
}

// Read the completed-discovery status flag without clearing it.
u8 LinkDmg07_HasPingStatus() {
    return LinkDmg07_PingStatusReady;
}

// Clear and report the completed-discovery flag; return zero if no new status exists.
u8 LinkDmg07_ConsumePingStatus() {
    if (LinkDmg07_PingStatusReady == 0) return 0;
    LinkDmg07_PingStatusReady = 0;
    return 1;
}

// Schedule the start handshake only from player one during discovery with
// its connection bit set. Return success means requested, not transfer-ready.
u8 LinkDmg07_RequestTransmission() {
    if (LinkDmg07_Phase != LINK_DMG07_PHASE_PING) {
        LinkDmg07_LastErrorCode = LINK_DMG07_ERR_STATE;
        return LINK_DMG07_ERR_STATE;
    }
    if (LinkDmg07_LocalSlot != 1) {
        LinkDmg07_LastErrorCode = LINK_DMG07_ERR_NOT_PLAYER1;
        return LINK_DMG07_ERR_NOT_PLAYER1;
    }
    if ((LinkDmg07_ConnectedMask & (u8)0x01) == 0) {
        LinkDmg07_LastErrorCode = LINK_DMG07_ERR_NOT_READY;
        return LINK_DMG07_ERR_NOT_READY;
    }
    LinkDmg07_StartRequested = 1;
    return LINK_DMG07_ERR_NONE;
}

// Request aligned recovery only in transfer mode. Repeated requests preserve
// an already pending or active restart rather than starting it again.
u8 LinkDmg07_RequestRestart() {
    if (LinkDmg07_Phase != LINK_DMG07_PHASE_TRANSFER) {
        LinkDmg07_LastErrorCode = LINK_DMG07_ERR_STATE;
        return LINK_DMG07_ERR_STATE;
    }
    if (LinkDmg07_RestartState == LINK_DMG07_RESTART_NONE) {
        LinkDmg07_RestartState = LINK_DMG07_RESTART_PENDING;
    }
    return LINK_DMG07_ERR_NONE;
}

// Return whether aligned recovery is absent, pending or sending control traffic.
u8 LinkDmg07_GetRestartState() {
    return LinkDmg07_RestartState;
}

// Set the application byte for a future packet boundary; an already armed SB
// byte is unchanged. Treat all-FF packets as reserved recovery traffic.
void __stackcall LinkDmg07_SetLocalByte(u8 value) {
    LinkDmg07_LocalByte = value;
}

// Report whether the initial transfer packet has been discarded to prime the pipeline.
u8 LinkDmg07_IsPipelinePrimed() {
    return LinkDmg07_PipelinePrimed;
}

// Inspect the one-packet receive mailbox without consuming it.
u8 LinkDmg07_HasPacket() {
    return LinkDmg07_PacketReady;
}

// Consume the latest four-byte packet. A nonnull destination must hold four
// bytes; null discards the packet. Packet storage remains after the flag clears.
u8 __stackcall LinkDmg07_ReadPacket(u8 *dst) {
    if (LinkDmg07_PacketReady == 0) return 0;
    if (dst != 0) {
        // Keep this as a sequential pointer copy.  Besides being smaller for a
        // fixed four-byte packet, it avoids making a serial hot-path copy
        // depend on general variable-index pointer lowering.
        *dst = LinkDmg07_RxPacket[0];
        dst++;
        *dst = LinkDmg07_RxPacket[1];
        dst++;
        *dst = LinkDmg07_RxPacket[2];
        dst++;
        *dst = LinkDmg07_RxPacket[3];
    }
    LinkDmg07_PacketReady = 0;
    return 1;
}

// Read one cached player byte using a one-based slot. Invalid slots return
// zero; the function neither requires readiness nor consumes the packet.
u8 __stackcall LinkDmg07_GetPacketSlot(u8 player_slot) {
    if ((player_slot == 0) || (player_slot > LINK_DMG07_MAX_PLAYERS)) return 0;
    return LinkDmg07_RxPacket[(__safe_index u8)(player_slot - 1)];
}

// Return the modulo-256 count of application packets started, excluding recovery traffic.
u8 LinkDmg07_GetSentSequence() {
    return LinkDmg07_SentSequence;
}

// Return the modulo-256 sequence associated with the most recently published packet.
u8 LinkDmg07_GetPacketSequence() {
    return LinkDmg07_PacketSequence;
}

// Return the saturating frame count since the last received serial byte.
u8 LinkDmg07_GetSilenceFrames() {
    return LinkDmg07_SilenceFrames;
}

// Return the saturating lifetime timeout count, preserved across protocol restarts.
u8 LinkDmg07_GetTimeoutCount() {
    return LinkDmg07_TimeoutCount;
}

// Return the saturating count of removed-player and previously-seen-adapter timeout events.
u8 LinkDmg07_GetDisconnectCount() {
    return LinkDmg07_DisconnectCount;
}

// Return the saturating protocol/overflow error count. Timeout and request
// validation paths use separate counters or latches and do not increment this value.
u8 LinkDmg07_GetErrorCount() {
    return LinkDmg07_ErrorCount;
}

// Return the latest error latch without clearing counters or restarting the protocol.
u8 LinkDmg07_LastError() {
    return LinkDmg07_LastErrorCode;
}

// Clear only the latest error code, preserving counters and protocol state.
void LinkDmg07_ClearError() {
    LinkDmg07_LastErrorCode = LINK_DMG07_ERR_NONE;
}
