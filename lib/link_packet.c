#pragma bank 0

#include "link.h"

extern u8 Link_Mode;
extern u8 Link_PacketState;
extern u8 Link_TxSeq;
extern u8 Link_RxSeq;
extern u8 Link_LastRx;

extern u8 Link_CurrentPacketCmd;
extern u8 Link_CurrentPacketLen;
extern u8 Link_CurrentPacketBuf[24];
extern u8 Link_CurrentPacketChecksum;
extern u8 Link_PendingAck;

extern u8 Link_ReceivedPacketReady;
extern u8 Link_ReceivedPacketCmd;
extern u8 Link_ReceivedPacketLen;
extern u8 Link_ReceivedPacketBuf[24];
extern u8 Link_ReceivedPacketChecksum;

extern u8 Link_PacketRetryCount;
extern u8 Link_PacketTimer;
extern u8 Link4_LastPacketPeer;
extern u8 Link4_ReceivedPacketReady[4];
extern u8 Link4_ReceivedPacketCmd[4];
extern u8 Link4_ReceivedPacketLen[4];
extern u8 Link4_ReceivedPacketBuf[96];

void Link_SetErrorInternal(u8 code);
u8 Link4_IsEnabledInternal();
u8 Link4_IsValidRemoteSlotInternal(u8 peer_slot);
u8 Link4_CurrentTransferPeerInternal();
u8 Link4_PacketOffset(u8 peer_slot, u8 index);
u8 __stackcall Link4_SelectPeer(u8 peer_slot);

#define LINK_PKT_SYNC          ((u8)0xA5)
#define LINK_PKT_ACK           ((u8)0x79)
#define LINK_PKT_NAK           ((u8)0x1F)
#define LINK_PKT_FILLER        ((u8)0xFF)
#define LINK_PKT_RETRY_MAX     ((u8)3)
#define LINK_PKT_TIMEOUT_TICKS ((u8)90)

#define LINK_PKT_STATE_IDLE          ((u8)0)
#define LINK_PKT_STATE_SEND_SYNC     ((u8)1)
#define LINK_PKT_STATE_SEND_CMD      ((u8)2)
#define LINK_PKT_STATE_SEND_LEN      ((u8)3)
#define LINK_PKT_STATE_SEND_PAYLOAD  ((u8)4)
#define LINK_PKT_STATE_SEND_CHECKSUM ((u8)5)
#define LINK_PKT_STATE_WAIT_ACK      ((u8)6)
#define LINK_PKT_STATE_RECV_SYNC     ((u8)7)
#define LINK_PKT_STATE_RECV_CMD      ((u8)8)
#define LINK_PKT_STATE_RECV_LEN      ((u8)9)
#define LINK_PKT_STATE_RECV_PAYLOAD  ((u8)10)
#define LINK_PKT_STATE_RECV_CHECKSUM ((u8)11)
#define LINK_PKT_STATE_SEND_ACK      ((u8)12)
#define LINK_PKT_STATE_ERROR         ((u8)13)

u8 Link4_PacketPeerReady(u8 peer_slot) {
    if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) return 0;
    return Link4_ReceivedPacketReady[(__safe_index u8)peer_slot];
}

void Link4_StoreReceivedPacketForPeer(u8 peer_slot) {
    u8 i;
    u8 offset;

    if (Link4_IsValidRemoteSlotInternal(peer_slot) == 0) return;

    Link4_ReceivedPacketReady[(__safe_index u8)peer_slot] = 1;
    Link4_ReceivedPacketCmd[(__safe_index u8)peer_slot] = Link_ReceivedPacketCmd;
    Link4_ReceivedPacketLen[(__safe_index u8)peer_slot] = Link_ReceivedPacketLen;
    for (i = 0; i < Link_ReceivedPacketLen; ++i) {
        offset = Link4_PacketOffset(peer_slot, i);
        Link4_ReceivedPacketBuf[(__safe_index u8)offset] = Link_ReceivedPacketBuf[(__safe_index u8)i];
    }
    Link4_LastPacketPeer = peer_slot;
}

u8 Link_PacketNeedsReceiveTransfer() {
    if (Link_IsBusy() != 0) return 0;

    if (Link_PacketState == LINK_PKT_STATE_IDLE) {
        if (Link_Mode == LINK_MODE_SLAVE && Link_ReceivedPacketReady == 0 && Link_PendingAck == 0) return 1;
        return 0;
    }

    if (Link_PacketState == LINK_PKT_STATE_RECV_CMD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_LEN) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_PAYLOAD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_CHECKSUM) return 1;
    return 0;
}

u8 Link_PacketStartReceiveTransfer() {
    u8 keep_state;

    if (Link_PacketNeedsReceiveTransfer() == 0) return 0;

    keep_state = Link_PacketState;
    if (Link_BeginTransfer(LINK_PKT_FILLER) != LINK_ERR_NONE) return 0;
    Link_PacketState = keep_state;
    return 1;
}

u8 Link_PacketIsSendState() {
    if (Link_PacketState == LINK_PKT_STATE_SEND_SYNC) return 1;
    if (Link_PacketState == LINK_PKT_STATE_SEND_CMD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_SEND_LEN) return 1;
    if (Link_PacketState == LINK_PKT_STATE_SEND_PAYLOAD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_SEND_CHECKSUM) return 1;
    if (Link_PacketState == LINK_PKT_STATE_WAIT_ACK) return 1;
    return 0;
}

u8 Link_PacketIsRecvState() {
    if (Link_PacketState == LINK_PKT_STATE_RECV_SYNC) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_CMD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_LEN) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_PAYLOAD) return 1;
    if (Link_PacketState == LINK_PKT_STATE_RECV_CHECKSUM) return 1;
    if (Link_PacketState == LINK_PKT_STATE_SEND_ACK) return 1;
    return 0;
}

void Link_PacketResetReceiveState() {
    Link_RxSeq = 0;
    Link_ReceivedPacketCmd = 0;
    Link_ReceivedPacketLen = 0;
    Link_ReceivedPacketChecksum = 0;
}

void Link_PacketResetSendState() {
    Link_TxSeq = 0;
    Link_PacketRetryCount = 0;
}

void Link_PacketGoIdle() {
    Link_PacketState = LINK_PKT_STATE_IDLE;
    Link_PacketTimer = 0;
    Link_TxSeq = 0;
    Link_RxSeq = 0;
}

void Link_PacketRetryOrFail(u8 error_code) {
    if ((u8)(Link_PacketRetryCount + 1) >= LINK_PKT_RETRY_MAX) {
        Link_SetErrorInternal(error_code);
        Link_PacketState = LINK_PKT_STATE_ERROR;
        return;
    }

    Link_PacketRetryCount++;
    Link_TxSeq = 0;
    Link_PacketState = LINK_PKT_STATE_SEND_SYNC;
    Link_PacketTimer = LINK_PKT_TIMEOUT_TICKS;
}

u8 Link_PacketBeginTransferByte(u8 out, u8 next_state) {
    if (Link_IsBusy() != 0) return 0;
    if (Link_BeginTransfer(out) != LINK_ERR_NONE) return 0;
    Link_PacketState = next_state;
    return 1;
}

u8 Link_PacketStartAckTransfer() {
    if (Link_PendingAck == 0) return 0;
    if (Link_IsBusy() != 0) return 0;
    if (Link_BeginTransfer(Link_PendingAck) != LINK_ERR_NONE) return 0;

    Link_PendingAck = 0;
    Link_PacketState = LINK_PKT_STATE_IDLE;
    return 1;
}

u8 Link_PacketStartNextTransfer() {
    u8 next_state;
    u8 payload_index;

    if (Link_IsBusy() != 0) return 0;

    if (Link_PacketState == LINK_PKT_STATE_SEND_SYNC) {
        return Link_PacketBeginTransferByte(LINK_PKT_SYNC, LINK_PKT_STATE_SEND_CMD);
    }

    if (Link_PacketState == LINK_PKT_STATE_SEND_CMD) {
        return Link_PacketBeginTransferByte(Link_CurrentPacketCmd, LINK_PKT_STATE_SEND_LEN);
    }

    if (Link_PacketState == LINK_PKT_STATE_SEND_LEN) {
        if (Link_CurrentPacketLen == 0) next_state = LINK_PKT_STATE_SEND_CHECKSUM;
        else next_state = LINK_PKT_STATE_SEND_PAYLOAD;
        return Link_PacketBeginTransferByte(Link_CurrentPacketLen, next_state);
    }

    if (Link_PacketState == LINK_PKT_STATE_SEND_PAYLOAD) {
        payload_index = Link_TxSeq;
        next_state = LINK_PKT_STATE_SEND_PAYLOAD;
        if ((u8)(payload_index + 1) >= Link_CurrentPacketLen) next_state = LINK_PKT_STATE_SEND_CHECKSUM;
        if (Link_PacketBeginTransferByte(Link_CurrentPacketBuf[(__safe_index u8)payload_index], next_state) == 0) return 0;
        Link_TxSeq++;
        return 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_SEND_CHECKSUM) {
        return Link_PacketBeginTransferByte(Link_CurrentPacketChecksum, LINK_PKT_STATE_WAIT_ACK);
    }

    if (Link_PacketState == LINK_PKT_STATE_WAIT_ACK) {
        return Link_PacketBeginTransferByte(LINK_PKT_FILLER, LINK_PKT_STATE_WAIT_ACK);
    }

    return 0;
}

u8 Link_PacketConsumeByte(u8 value) {
    if (Link_PacketState == LINK_PKT_STATE_WAIT_ACK) {
        if (value == LINK_PKT_ACK) {
            Link_PacketResetSendState();
            Link_PacketGoIdle();
            return 1;
        }

        if (value == LINK_PKT_NAK) {
            Link_PacketRetryOrFail(LINK_ERR_PROTOCOL);
            return 1;
        }

        return 0;
    }

    if (Link_PacketIsSendState() != 0) {
        return 0;
    }

    if (Link_PacketState == LINK_PKT_STATE_IDLE) {
        if (value != LINK_PKT_SYNC) return 0;

        if (Link4_IsEnabledInternal() != 0) {
            if (Link4_PacketPeerReady(Link4_CurrentTransferPeerInternal()) != 0) {
                Link_SetErrorInternal(LINK_ERR_OVERFLOW);
                Link_PendingAck = LINK_PKT_NAK;
                Link_PacketState = LINK_PKT_STATE_SEND_ACK;
                return 1;
            }
        } else if (Link_ReceivedPacketReady != 0) {
            Link_SetErrorInternal(LINK_ERR_OVERFLOW);
            Link_PendingAck = LINK_PKT_NAK;
            Link_PacketState = LINK_PKT_STATE_SEND_ACK;
            return 1;
        }

        Link_PacketResetReceiveState();
        Link_PacketState = LINK_PKT_STATE_RECV_SYNC;
        Link_PacketState = LINK_PKT_STATE_RECV_CMD;
        return 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_RECV_CMD) {
        Link_ReceivedPacketCmd = value;
        Link_ReceivedPacketChecksum = value;
        Link_PacketState = LINK_PKT_STATE_RECV_LEN;
        return 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_RECV_LEN) {
        if (value > LINK_PKT_MAX_PAYLOAD) {
            Link_SetErrorInternal(LINK_ERR_PROTOCOL);
            Link_PendingAck = LINK_PKT_NAK;
            Link_PacketState = LINK_PKT_STATE_SEND_ACK;
            return 1;
        }

        Link_ReceivedPacketLen = value;
        Link_ReceivedPacketChecksum = (u8)(Link_ReceivedPacketChecksum ^ value);
        Link_RxSeq = 0;

        if (value == 0) Link_PacketState = LINK_PKT_STATE_RECV_CHECKSUM;
        else Link_PacketState = LINK_PKT_STATE_RECV_PAYLOAD;
        return 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_RECV_PAYLOAD) {
        Link_ReceivedPacketBuf[(__safe_index u8)Link_RxSeq] = value;
        Link_ReceivedPacketChecksum = (u8)(Link_ReceivedPacketChecksum ^ value);
        Link_RxSeq++;

        if (Link_RxSeq >= Link_ReceivedPacketLen) Link_PacketState = LINK_PKT_STATE_RECV_CHECKSUM;
        return 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_RECV_CHECKSUM) {
        if (value == Link_ReceivedPacketChecksum) {
            Link_ReceivedPacketReady = 1;
            if (Link4_IsEnabledInternal() != 0) {
                Link4_StoreReceivedPacketForPeer(Link4_CurrentTransferPeerInternal());
            } else {
                Link4_LastPacketPeer = LINK4_INVALID_SLOT;
            }
            Link_PendingAck = LINK_PKT_ACK;
        } else {
            Link_SetErrorInternal(LINK_ERR_PROTOCOL);
            Link_ReceivedPacketReady = 0;
            Link_PendingAck = LINK_PKT_NAK;
        }

        Link_RxSeq = 0;
        Link_PacketState = LINK_PKT_STATE_SEND_ACK;
        return 1;
    }

    return 0;
}

void Link_PacketHandleTimeout() {
    if (Link_PacketState == LINK_PKT_STATE_WAIT_ACK) {
        if (Link_LastRx == LINK_PKT_FILLER) Link_PacketRetryOrFail(LINK_ERR_DISCONNECT);
        else Link_PacketRetryOrFail(LINK_ERR_TIMEOUT);
        return;
    }

    if (Link_PacketIsSendState() != 0) {
        Link_PacketRetryOrFail(LINK_ERR_TIMEOUT);
        return;
    }

    if (Link_PacketIsRecvState() != 0) {
        Link_SetErrorInternal(LINK_ERR_TIMEOUT);
        Link_PendingAck = 0;
        Link_PacketState = LINK_PKT_STATE_ERROR;
    }
}

u8 __stackcall Link_SendPacket(const u8 *data, u8 len, u8 cmd) {
    u8 i;
    u8 checksum = (u8)(cmd ^ len);

    if (len > LINK_PKT_MAX_PAYLOAD) {
        Link_SetErrorInternal(LINK_ERR_PROTOCOL);
        return LINK_ERR_PROTOCOL;
    }

    if (len != 0 && data == 0) {
        Link_SetErrorInternal(LINK_ERR_PROTOCOL);
        return LINK_ERR_PROTOCOL;
    }

    if (Link_PacketState != LINK_PKT_STATE_IDLE || Link_PendingAck != 0) {
        Link_SetErrorInternal(LINK_ERR_BUSY);
        return LINK_ERR_BUSY;
    }

    Link_CurrentPacketCmd = cmd;
    Link_CurrentPacketLen = len;

    for (i = 0; i < len; ++i) {
        Link_CurrentPacketBuf[(__safe_index u8)i] = data[(__safe_index u8)i];
        checksum = (u8)(checksum ^ data[(__safe_index u8)i]);
    }

    Link_CurrentPacketChecksum = checksum;
    Link_TxSeq = 0;
    Link_RxSeq = 0;
    Link_PacketRetryCount = 0;
    Link_PacketTimer = LINK_PKT_TIMEOUT_TICKS;
    Link_PacketState = LINK_PKT_STATE_SEND_SYNC;
    return LINK_ERR_NONE;
}

void Link_PollPacket() {
    u8 progress = 0;
    u8 value = 0;

    Link_Poll();

    if (Link_PacketState == LINK_PKT_STATE_ERROR) {
        Link_PacketGoIdle();
        return;
    }

    while (Link_TryReadByte(&value) != 0) {
        if (Link_PacketConsumeByte(value) != 0) progress = 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_SEND_ACK) {
        if (Link_PacketStartAckTransfer() != 0) progress = 1;
    } else if (Link_PacketIsSendState() != 0) {
        if (Link_PacketStartNextTransfer() != 0) progress = 1;
    } else {
        if (Link_PacketStartReceiveTransfer() != 0) progress = 1;
    }

    if (Link_PacketState == LINK_PKT_STATE_IDLE) {
        Link_PacketTimer = 0;
        return;
    }

    if (progress != 0) {
        Link_PacketTimer = LINK_PKT_TIMEOUT_TICKS;
        return;
    }

    if (Link_PacketTimer != 0) Link_PacketTimer--;
    if (Link_PacketTimer == 0) Link_PacketHandleTimeout();
}

u8 Link_HasPacket() {
    return Link_ReceivedPacketReady;
}

u8 __stackcall Link_ReadPacket(u8 *cmd, u8 *len, u8 *dst) {
    u8 i;
    u8 peer_slot = Link4_LastPacketPeer;

    if (Link_ReceivedPacketReady == 0) return 0;

    if (cmd != 0) *cmd = Link_ReceivedPacketCmd;
    if (len != 0) *len = Link_ReceivedPacketLen;

    if (dst != 0) {
        for (i = 0; i < Link_ReceivedPacketLen; ++i) {
            dst[(__safe_index u8)i] = Link_ReceivedPacketBuf[(__safe_index u8)i];
        }
    }

    Link_ReceivedPacketReady = 0;
    if (Link4_IsValidRemoteSlotInternal(peer_slot) != 0) {
        Link4_ReceivedPacketReady[(__safe_index u8)peer_slot] = 0;
    }
    Link4_LastPacketPeer = LINK4_INVALID_SLOT;
    return 1;
}

u8 __stackcall Link4_SendPacketTo(u8 peer_slot, const u8 *data, u8 len, u8 cmd) {
    u8 err = Link4_SelectPeer(peer_slot);
    if (err != LINK_ERR_NONE) return err;
    return Link_SendPacket(data, len, cmd);
}

void Link4_PollPacket() {
    Link_PollPacket();
}

u8 __stackcall Link4_HasPacketFrom(u8 peer_slot) {
    return Link4_PacketPeerReady(peer_slot);
}

u8 __stackcall Link4_ReadPacketFrom(u8 peer_slot, u8 *cmd, u8 *len, u8 *dst) {
    u8 i;
    u8 offset;

    if (Link4_PacketPeerReady(peer_slot) == 0) return 0;

    if (cmd != 0) *cmd = Link4_ReceivedPacketCmd[(__safe_index u8)peer_slot];
    if (len != 0) *len = Link4_ReceivedPacketLen[(__safe_index u8)peer_slot];

    if (dst != 0) {
        for (i = 0; i < Link4_ReceivedPacketLen[(__safe_index u8)peer_slot]; ++i) {
            offset = Link4_PacketOffset(peer_slot, i);
            dst[(__safe_index u8)i] = Link4_ReceivedPacketBuf[(__safe_index u8)offset];
        }
    }

    Link4_ReceivedPacketReady[(__safe_index u8)peer_slot] = 0;
    if (Link4_LastPacketPeer == peer_slot) {
        Link_ReceivedPacketReady = 0;
        Link4_LastPacketPeer = LINK4_INVALID_SLOT;
    }
    return 1;
}
