use std::any::Any;

use crate::constants::{
    KcpError, KcpLogFlags, KcpProbeFlags, IKCP_DEADLINK, IKCP_FASTACK_LIMIT, IKCP_INTERVAL,
    IKCP_MTU_DEF, IKCP_OVERHEAD, IKCP_RTO_DEF, IKCP_RTO_MIN, IKCP_THRESH_INIT, IKCP_WND_RCV,
    IKCP_WND_SND,
};

macro_rules! ikcp_log {
    ($kcp: expr, $mask:expr, $($arg:tt)+) => {
        if $kcp.canlog($mask) {
            let s = format!($($arg)*);
            $kcp.__log(s);
        }

    }

}

#[derive(Default)]
pub struct KcpControl {
    /// conversation id
    /// The conversation id is used to identify each connection, which will not change
    /// during the connection life-time.
    ///
    /// It is represented by a 32 bits integer which is given at the moment the KCP
    /// control block (aka. struct ikcpcb, or kcp object) has been created. Each
    /// packet sent out will carry the conversation id in the first 4 bytes and a
    /// packet from remote endpoint will not be accepted if it has a different
    /// conversation id.
    ///
    /// The value can be any random number, but in practice, both side between a
    /// connection will have many KCP objects (or control block) storing in the
    /// containers like a map or an array. A index is used as the key to look up one
    /// KCP object from the container.
    ///
    /// So, the higher 16 bits of conversation id can be used as caller's index while
    /// the lower 16 bits can be used as callee's index. KCP will not handle
    /// handshake, and the index in both side can be decided and exchanged after
    /// connection establish.
    ///
    /// When you receive and accept a remote packet, the local index can be extracted
    /// from the conversation id and the kcp object which is in charge of this
    /// connection can be find out from your map or array.
    conversation_id: u32,
    pub mtu: u32,
    pub mss: u32,
    pub state: u32,
    pub snd_una: u32,
    pub snd_nxt: u32,
    pub rcv_nxt: u32,
    pub ts_recent: u32,
    pub ts_lastack: u32,
    /// slow start threshold
    pub ssthresh: u32,
    pub rx_rttval: i32,
    pub rx_srtt: i32,
    /// Retransmission timeout (ms)
    pub rx_rto: u32,
    /// Minimum retransmission timeout (ms)
    pub rx_minrto: u32,
    pub send_window: u32,
    pub recv_window: u32,
    pub rmt_wnd: u32,
    pub cwnd: u32,
    pub probe: KcpProbeFlags,
    pub current: u32,
    /// Internal update interval in milliseconds.
    ///
    /// This controls how frequently KCP checks for packet resends, window updates,
    /// and other internal operations. Smaller values make KCP more responsive but
    /// consume more CPU. Typical values:
    /// - 10-30ms for real-time applications (e.g. games)
    /// - 100ms for normal applications (default)
    /// - 200+ms for delay-tolerant applications
    pub interval: u32,
    pub ts_flush: u32,
    pub xmit: u32,
    pub nsnd_buf: u32,
    pub nodelay: u32,
    pub updated: u32,
    pub ts_probe: u32,
    pub probe_wait: u32,
    /// Dead link detection counter.
    ///
    /// Incremented when no valid packets are received, reset on successful communication.
    /// When reaches IKCP_DEADLINK (default 20), the connection is considered broken.
    pub dead_link: u32,
    pub incr: u32,
    pub snd_queue: Vec<Segment>,
    pub rcv_queue: Vec<Segment>,
    pub snd_buf: Vec<Segment>,
    pub rcv_buf: Vec<Segment>,
    pub acklist: Vec<u32>,
    pub ackcount: u32,
    pub ackblock: u32,
    pub fastresend: i32,
    /// Enable logging.
    write_log: bool,
    log_mask: KcpLogFlags,
    /// Fast ACK threshold for triggering fast retransmit.
    ///
    /// When receiving this number of duplicate ACKs, KCP will trigger fast retransmit
    /// without waiting for timeout. Default is 5 (IKCP_FASTACK_LIMIT).
    pub fastlimit: u32,

    /// Disable congestion window control when non-zero.
    ///
    /// When set to true, KCP will send data as fast as possible without
    /// congestion control. Useful for latency-sensitive applications that
    /// can tolerate packet loss. Default is `false` (congestion control enabled).
    pub nocwnd: bool,
    pub streaming_mode: bool,
    pub callback: Option<Box<dyn KcpCallBack>>,
    user_data: Option<Box<dyn Any>>,
    buffer: Vec<u8>,
}

impl KcpControl {
    /// Create a new KCP control block on the heap
    ///
    /// # Arguments
    /// * `conversation_id` - Unique identifier for this KCP connection
    /// * `user_data` - Optional user-defined data to associate with this KCP instance
    ///
    /// # Returns
    /// Box containing the initialized KCP control block
    pub fn new_alloc(conversation_id: u32, user_data: Option<Box<dyn Any>>) -> Box<Self> {
        let mut x = Box::new(Self::default());
        x.init(conversation_id, user_data);
        x
    }

    /// Create a new KCP control block on the stack
    ///
    /// # Arguments
    /// * `conversation_id` - Unique identifier for this KCP connection
    /// * `user_data` - Optional user-defined data to associate with this KCP instance
    ///
    /// # Returns
    /// Initialized KCP control block
    pub fn new_on_stack(conversation_id: u32, user_data: Option<Box<dyn Any>>) -> Self {
        let mut x = Self::default();
        x.init(conversation_id, user_data);
        x
    }

    /// Get the conversation ID of this KCP instance
    ///
    /// # Returns
    /// The unique conversation ID assigned during creation
    pub const fn conversation_id(&self) -> u32 {
        self.conversation_id
    }

    /// Initialize KCP control block with default parameters
    ///
    /// # Arguments
    /// * `conversation_id` - Unique identifier for this KCP connection
    /// * `user_data` - Optional user-defined data to associate with this KCP instance
    ///
    /// # Note
    /// This sets all KCP parameters to their default values:
    fn init(&mut self, conversation_id: u32, user_data: Option<Box<dyn Any>>) {
        self.conversation_id = conversation_id;
        self.user_data = user_data;
        self.send_window = IKCP_WND_SND;
        self.recv_window = IKCP_WND_RCV;
        self.rmt_wnd = IKCP_WND_RCV;
        self.mtu = IKCP_MTU_DEF;
        self.update_mss();
        self.buffer
            .resize((self.mtu + IKCP_OVERHEAD) as usize * 3, 0);

        self.rx_rto = IKCP_RTO_DEF;
        self.rx_minrto = IKCP_RTO_MIN;
        self.interval = IKCP_INTERVAL;
        self.ssthresh = IKCP_THRESH_INIT;
        self.fastlimit = IKCP_FASTACK_LIMIT;
        self.dead_link = IKCP_DEADLINK;
    }

    /// update mss by mtu
    const fn update_mss(&mut self) {
        self.mss = self.mtu - IKCP_OVERHEAD;
    }

    /// Set the callback handler for this KCP instance
    ///
    /// # Arguments
    /// * `callback` - Box containing the callback implementation that handles:
    ///
    /// # Note
    /// The callback must implement both `Send` and `Sync` traits to be thread-safe
    pub fn set_callback(&mut self, callback: Box<dyn KcpCallBack>) {
        self.callback = Some(callback);
    }

    /// Receive data from KCP protocol
    ///
    /// user/upper level interface
    ///
    /// # Arguments
    /// * `data` - Optional mutable buffer to store received data
    /// * `is_peek` - If true, only peek data without removing from queue
    ///
    /// # Returns
    /// Number of bytes received or error
    ///
    /// # Errors
    /// - `QueueEmpty`: No data available in receive queue
    /// - `BufferTooSmall`: Provided buffer is smaller than message size
    pub fn receive(
        &mut self,
        mut data: Option<&mut [u8]>,
        is_peek: bool,
    ) -> Result<usize, KcpError> {
        if self.rcv_queue.is_empty() {
            return Err(KcpError::QueueEmpty);
        }

        let peeksize = self.peek_size()?;
        if let Some(buf) = &data {
            if peeksize > buf.len() {
                return Err(KcpError::BufferTooSmall);
            }
        }

        let mut total_len = 0;
        let recover = self.rcv_queue.len() >= self.recv_window as usize;

        let mut copy_offset = 0;

        // merge fragments in receive queue
        let mut i = 0;
        while i < self.rcv_queue.len() {
            let seg = &self.rcv_queue[i];

            // Copy data if buffer provided
            if let Some(d) = data.as_mut() {
                d[copy_offset..copy_offset + seg.len as usize]
                    .copy_from_slice(&seg.data[..seg.len as usize]);
                copy_offset += seg.len as usize;
            }

            total_len += seg.len as usize;
            let is_last_fragment = seg.frg == 0;

            ikcp_log!(self, KcpLogFlags::DATA_RECV, "recv sn={}", seg.sn);

            if !is_peek {
                self.rcv_queue.remove(i);
            } else {
                i += 1;
            }
            if is_last_fragment {
                break;
            }
        }

        assert_eq!(peeksize, total_len);

        // Move data from receive buffer to queue if space available
        while !self.rcv_buf.is_empty() && self.rcv_queue.len() < self.recv_window as usize {
            let seg = &self.rcv_buf[0];
            if seg.sn == self.rcv_nxt {
                let seg = self.rcv_buf.remove(0);
                self.rcv_queue.push(seg);
                self.rcv_nxt += 1;
            } else {
                break;
            }
        }

        // fast recover
        // Trigger window update if needed
        if self.rcv_queue.len() < self.recv_window as usize && recover {
            // ready to send back IKCP_CMD_WINS in ikcp_flush
            // tell remote my window size
            self.probe |= KcpProbeFlags::ASK_TELL;
        }

        Ok(total_len)
    }

    /// Send data through KCP protocol
    ///
    /// # Arguments
    /// * `data` - The data buffer to be sent
    ///
    /// # Returns
    /// - Ok(usize): Number of bytes successfully sent
    /// - Err(KcpError::WindowFull): When data exceeds receive window size
    ///
    /// # Note
    /// This method handles both streaming and packet mode, with flow control
    pub fn send(&mut self, data: &[u8]) -> Result<usize, KcpError> {
        assert!(self.mss > 0);

        let mut data_ptr = data;
        let mut sent = 0;
        let mut count;

        // In streaming mode, try to append data to the last segment if possible
        if self.streaming_mode {
            if !self.snd_queue.is_empty() {
                let seg = self.snd_queue.last_mut().unwrap();
                if seg.len < self.mss {
                    let capacity = (self.mss - seg.len) as usize;
                    let extend = if data.len() < capacity {
                        data.len()
                    } else {
                        capacity
                    };

                    // Append data to existing segment
                    seg.data.extend_from_slice(&data[..extend]);
                    data_ptr = &data[extend..];
                    seg.len += extend as u32;
                    seg.frg = 0; // Mark as non-fragmented in streaming mode

                    sent = extend;
                }
            }

            // Return early if all data was appended to existing segment
            if data_ptr.len() == 0 {
                return Ok(sent);
            }
        }

        // Calculate number of segments needed for remaining data
        if data_ptr.len() <= self.mss as usize {
            count = 1; // Single segment
        } else {
            // Round up division to get total segments needed
            count = (data_ptr.len() + self.mss as usize - 1) / self.mss as usize;
        }

        // Check if total segments exceed receive window size
        if count >= self.recv_window as usize {
            // In streaming mode, return partial success if some data was sent
            if self.streaming_mode && sent > 0 {
                return Ok(sent);
            }

            // Return error if window is full and no data was sent
            return Err(KcpError::WindowFull);
        }

        // Ensure at least one segment will be created
        if count == 0 {
            count = 1;
        }

        // Split remaining data into segments and add to send queue
        for i in 0..count {
            // Determine segment size (either MSS or remaining data size)
            let size = if data_ptr.len() > self.mss as usize {
                self.mss as usize
            } else {
                data_ptr.len()
            };

            // Create new segment with calculated size
            let mut seg = Segment::new(size);

            // Copy data to segment if there's data remaining
            if data_ptr.len() > 0 {
                seg.data[..size].copy_from_slice(&data_ptr[..size]);
            }

            // Set segment length
            seg.len = size as u32;

            // Set fragment number (only in non-streaming mode)
            seg.frg = if !self.streaming_mode {
                // Fragments are numbered in reverse order (last fragment is 0)
                (count - i - 1) as u32
            } else {
                // No fragmentation in streaming mode
                0
            };

            // Add segment to send queue
            self.snd_queue.push(seg);

            // Advance data pointer
            data_ptr = &data_ptr[size..];

            // Update total bytes sent
            sent += size;
        }

        // Return total bytes successfully queued for sending
        return Ok(sent);
    }

    pub fn set_logging(&mut self, enable: bool) {
        self.write_log = enable;
    }
    pub fn set_log_mask(&mut self, mask: KcpLogFlags) {
        self.log_mask = mask;
    }

    pub const fn log_mask(&self) -> &KcpLogFlags {
        &self.log_mask
    }

    pub const fn write_log(&self) -> bool {
        self.write_log
    }

    /// Get the size of next message in receive queue without removing it
    ///
    /// # Returns
    /// - Ok(usize): Size of next complete message in bytes
    /// - Err(KcpError::QueueEmpty): Receive queue is empty
    /// - Err(KcpError::IncompleteMessage): Message fragments are incomplete
    ///
    /// # Note
    /// This checks both single-segment messages and multi-segment fragmented messages
    pub fn peek_size(&self) -> Result<usize, KcpError> {
        if self.rcv_queue.is_empty() {
            return Err(KcpError::QueueEmpty);
        }

        let first_seg = &self.rcv_queue[0];

        // Single segment message
        if first_seg.frg == 0 {
            return Ok(first_seg.len as usize);
        }

        // Check if all fragments are present
        if self.rcv_queue.len() < (first_seg.frg + 1) as usize {
            return Err(KcpError::IncompleteMessage);
        }

        // Calculate total length of fragmented message
        let mut total_len = 0;
        for seg in &self.rcv_queue {
            total_len += seg.len as usize;
            if seg.frg == 0 {
                break;
            }
        }

        Ok(total_len)
    }

    fn __log(&self, s: String) {
        if let Some(ref callback) = self.callback {
            callback.writelog(&s, self, self.user_data.as_ref());
        }
    }

    /// Check if logging is enabled and the log mask matches the specified log flags
    ///
    /// # Arguments
    /// * `mask` - The log flags to check against
    fn canlog(&self, mask: KcpLogFlags) -> bool {
        self.log_mask.intersects(mask) && self.write_log && self.callback.is_some()
    }
}

/// Callback trait for KCP protocol events
///
/// Implement this trait to handle KCP output and logging events.
/// The trait requires both Send and Sync for thread safety.
#[allow(unused)]
pub trait KcpCallBack: Send + Sync {
    /// Called when KCP needs to send data packets
    ///
    /// # Arguments
    /// * `buf` - The data buffer to be sent
    /// * `kcp` - Reference to the KCP control block
    /// * `user` - Optional user data associated with the KCP instance
    ///
    /// # Note
    /// This is the core output function that should implement actual packet sending logic.
    /// Typically this would send the data over UDP or other transport protocol.
    fn output(&self, buf: &[u8], kcp: &mut KcpControl, user: Option<&Box<dyn Any>>) {}

    /// Called when KCP wants to output log messages
    ///
    /// # Arguments
    /// * `log` - The log message
    /// * `kcp` - Reference to the KCP control block
    /// * `user` - Optional user data associated with the KCP instance
    ///
    /// # Note
    /// This is optional and can be left unimplemented if logging is not needed.
    fn writelog(&self, log: &str, kcp: &KcpControl, user: Option<&Box<dyn Any>>) {}
}

#[derive(Clone, Default)]
pub struct Segment {
    pub conv: u32,
    pub cmd: u32,
    pub frg: u32,
    pub wnd: u32,
    pub ts: u32,
    pub sn: u32,
    pub una: u32,
    pub len: u32,
    pub resendts: u32,
    pub rto: u32,
    pub fastack: u32,
    pub xmit: u32,
    pub data: Vec<u8>, // Using Vec<u8> to represent the flexible array member `char data[1]`
}

impl Segment {
    pub fn new(data_size: usize) -> Self {
        let mut x = Self::default();
        x.data.resize(data_size, 0);
        x
    }
}
