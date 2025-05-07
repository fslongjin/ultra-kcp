use bitflags::bitflags;

// No delay minimum retransmission timeout
pub const IKCP_RTO_NDL: u32 = 30;

// Normal minimum retransmission timeout
pub const IKCP_RTO_MIN: u32 = 100;

// Default retransmission timeout
pub const IKCP_RTO_DEF: u32 = 200;

// Maximum retransmission timeout
pub const IKCP_RTO_MAX: u32 = 60000;

// Command enum
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Push data
    Push = 81,
    /// ack
    Ack = 82,
    /// window probe (ask)
    Wask = 83,
    /// window size (tell)
    Wins = 84,
}

impl TryFrom<u32> for Command {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            81 => Ok(Command::Push),
            82 => Ok(Command::Ack),
            83 => Ok(Command::Wask),
            84 => Ok(Command::Wins),
            _ => Err("Invalid command value"),
        }
    }
}

impl From<Command> for u32 {
    fn from(command: Command) -> Self {
        command as u32
    }
}

bitflags! {
    #[derive(Default)]
    pub struct KcpProbeFlags: u32 {
        const NONE = 0;
        // Need to send IKCP_CMD_WASK
        const ASK_SEND = 1 << 0;
        // Need to send IKCP_CMD_WINS
        const ASK_TELL = 1 << 1;
    }
}

// Send window size
pub const IKCP_WND_SND: u32 = 32;

// Receive window size (must be >= max fragment size)
pub const IKCP_WND_RCV: u32 = 128;

// Default maximum transmission unit
pub const IKCP_MTU_DEF: u32 = 1400;

// Fast acknowledgment threshold
pub const IKCP_ACK_FAST: u32 = 3;

// Default interval for protocol updates (ms)
pub const IKCP_INTERVAL: u32 = 100;

// Protocol overhead size
pub const IKCP_OVERHEAD: u32 = 24;

// Dead link threshold
pub const IKCP_DEADLINK: u32 = 20;

// Initial congestion window threshold
pub const IKCP_THRESH_INIT: u32 = 2;

// Minimum congestion window threshold
pub const IKCP_THRESH_MIN: u32 = 2;

// Initial probe interval for window size (7 seconds)
pub const IKCP_PROBE_INIT: u32 = 7000;

// Maximum probe interval for window size (120 seconds)
pub const IKCP_PROBE_LIMIT: u32 = 120000;

// Maximum times to trigger fast acknowledgment
pub const IKCP_FASTACK_LIMIT: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcpError {
    QueueEmpty,
    /// Buffer is too small to hold the data
    BufferTooSmall,
    /// Message fragments are incomplete
    IncompleteMessage,
    /// Window size is too small to hold the data
    WindowFull,
}

bitflags! {
    #[derive(Default)]
    pub struct KcpLogFlags: u32 {
        /// Enable basic output logging
        const OUTPUT = 1 << 0;
        /// Enable basic input logging
        const INPUT = 1 << 1;
        /// Log outgoing data segments
        const DATA_SEND = 1 << 2;
        /// Log incoming data segments
        const DATA_RECV = 1 << 3;
        /// Log incoming data packets
        const IN_DATA = 1 << 4;
        /// Log incoming ACK packets
        const IN_ACK = 1 << 5;
        /// Log incoming window probe requests
        const IN_PROBE = 1 << 6;
        /// Log incoming window size updates
        const IN_WINS = 1 << 7;
        /// Log outgoing data packets
        const OUT_DATA = 1 << 8;
        /// Log outgoing ACK packets
        const OUT_ACK = 1 << 9;
        /// Log outgoing window probe requests
        const OUT_PROBE = 1 << 10;
        /// Log outgoing window size updates
        const OUT_WINS = 1 << 11;
    }
}
