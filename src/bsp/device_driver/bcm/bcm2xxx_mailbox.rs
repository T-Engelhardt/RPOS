use core::{mem::size_of, ptr};

use crate::{
    bsp::device_driver::common::MMIODerefWrapper, cpu, debug, driver, gpu::*, info,
    synchronization, synchronization::NullLock,
};

use tock_registers::{
    fields::FieldValue,
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

const BUFFER_LENGTH: usize = 100;
/// Buffer for the Mail message send and recv
///
/// this ensure the message is not on the stack
/// and can be written and read everywhere
///
/// # Safety
///
/// - fn that access BUFFER are proteced with a LOCK
static mut BUFFER: [u32; BUFFER_LENGTH] = [0; BUFFER_LENGTH];

register_bitfields! {
    u32,

    READ [
        CHANNEL OFFSET(0) NUMBITS(4) [],
        DATA OFFSET(4) NUMBITS(28) [],
    ],

    STATUS [
        EMPTY OFFSET(30) NUMBITS(1) [],
        FULL OFFSET(31) NUMBITS(1) [],
    ],

    WRITE [
        CHANNEL OFFSET(0) NUMBITS(4) [
            MAIL_POWER    = 0x0, // Mailbox Channel 0: Power Management Interface
            MAIL_FB       = 0x1, // Mailbox Channel 1: Frame Buffer
            MAIL_VUART    = 0x2, // Mailbox Channel 2: Virtual UART
            MAIL_VCHIQ    = 0x3, // Mailbox Channel 3: VCHIQ Interface
            MAIL_LEDS     = 0x4, // Mailbox Channel 4: LEDs Interface
            MAIL_BUTTONS  = 0x5, // Mailbox Channel 5: Buttons Interface
            MAIL_TOUCH    = 0x6, // Mailbox Channel 6: Touchscreen Interface
            MAIL_COUNT    = 0x7, // Mailbox Channel 7: Counter
            MAIL_TAGS     = 0x8, // Mailbox Channel 8: Tags (ARM to VC
        ],
        DATA OFFSET(4) NUMBITS(28) [],
    ],
}

register_structs! {
    #[allow(non_snake_case)]
    pub RegisterBlock {
        (0x00 => READ: ReadOnly<u32, READ::Register>),
        (0x04 => _reserved1),
        (0x18 => STATUS: ReadOnly<u32, STATUS::Register>),
        (0x1C => _reserved2), // CONFIG
        (0x20 => WRITE: ReadWrite<u32, WRITE::Register>),
        (0x24 => @END),
    }
}

/// Abstraction for the associated MMIO registers.
type Registers = MMIODerefWrapper<RegisterBlock>;

struct MailBoxInner {
    registers: Registers,
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Representation of the Mailbox.
pub struct MailBox {
    inner: NullLock<MailBoxInner>,
}

impl MailBoxInner {
    pub const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            registers: Registers::new(mmio_start_addr),
        }
    }

    // TODO
    // check return code
    pub fn request_framebuffer(&self) -> Option<Display> {
        {
            #[rustfmt::skip]
            let msg: [u32; 28] = [
                112,                            // length
                0,                              // request/response code
                0x00048003, 8, 0, 1920, 1080,   // sets the screen size to
                0x00048004, 8, 0, 1920, 1080,   // sets the virtual screen size
                0x00048005, 4, 0, 32,           // sets the depth to 32bits
                0x00048006, 4, 0, 1,            // set pixel order to RGB
                0x00040007, 4, 0, 2,            // set alpha channel => ignore
                0,                              // end tag
                0, 0, 0,                        // padding
            ];

            self.send_mail(&msg, WRITE::CHANNEL::MAIL_TAGS);
        }

        // wait until message on given channel is received
        while let Err(_) = self.recv_mail(WRITE::CHANNEL::MAIL_TAGS) {}

        let mut result = Display {
            width: unsafe { BUFFER[10] },
            height: unsafe { BUFFER[11] },
            depth: ColorDepth::determine_depth(unsafe { BUFFER[15] }, unsafe { BUFFER[19] == 0 }),
            fp_ptr: None,
            fp_len: 0,
        };

        {
            #[rustfmt::skip]
            let msg: [u32; 8] = [
                32,
                0,
                0x00040001, 8, 0, 16, 0,
                0,
            ];

            self.send_mail(&msg, WRITE::CHANNEL::MAIL_TAGS);
        }

        while let Err(_) = self.recv_mail(WRITE::CHANNEL::MAIL_TAGS) {}

        // convert videocore mapped addr to arm addr
        result.fp_ptr = Some((unsafe { BUFFER[5] } & 0x3FFFFFFF) as *const u32);
        result.fp_len = unsafe { BUFFER[6] } as usize;

        Some(result)
    }

    // calc padding (nr. of zeros as u32[4bytes]) so the size is  16 byte aligned
    fn calc_padding<T>(&self, len: usize) -> usize {
        // https://en.wikipedia.org/wiki/Data_structure_alignment
        // padding = (align - (offset mod align)) mod align
        ((16 - ((size_of::<T>() * len) % 16)) % 16) / size_of::<T>()
    }

    // copy message from the stack to the static buffer
    fn copy_to_buffer(&self, len: usize, src: &[u32]) {
        if self.calc_padding::<u32>(BUFFER_LENGTH) != 0 {
            panic!("Buffer not 16 bit aligned")
        }

        if self.calc_padding::<u32>(len) != 0 {
            panic!("msg/src not 16 bit aligned")
        }

        unsafe {
            BUFFER[..len].copy_from_slice(src);
        }
    }

    // sends message to mailbox
    // msg is copied to buffer
    fn send_mail(&self, msg: &[u32], channel: FieldValue<u32, WRITE::Register>) {
        // make sure that the addr is not on the stack
        self.copy_to_buffer(msg.len(), msg);

        info!("Sending Mail on channel {}", channel.value);
        // debug print
        debug!(
            "Addr: {:?} ; Len: {:#010x}",
            unsafe { BUFFER.as_mut_ptr() },
            msg.len() * size_of::<u32>(),
        );

        for i in 0..msg.len() {
            let n = unsafe { ptr::read_volatile(&BUFFER[i]) };
            debug!("{:#010x}", n);
        }

        // write data
        // channel + address bit shif by length of the channel
        // c c c c | a a ... a
        // thats why the address needs to be a 16 byte aligned buffer
        self.registers
            .WRITE
            .write(channel + WRITE::DATA.val(unsafe { BUFFER.as_ptr() } as u32 >> 4));
    }

    // blocks until message is received
    // returns OK with channel id if it received the msg on the given channel or ERR with incorrect channel
    fn recv_mail(&self, channel: FieldValue<u32, WRITE::Register>) -> Result<u32, u32> {
        // wait for data
        info!("Waiting Mail on channel {}", channel.value);
        while self.registers.STATUS.matches_all(STATUS::EMPTY::SET) {
            cpu::nop();
        }

        // The callee is not allowed to return a different buffer address, this allows the caller to make independent asynchronous requests.
        // Thats why we dont need to check the response data since its the BUFFER Addr
        let recv_channel = self.registers.READ.read(READ::CHANNEL);
        info!("Received Mail on channel {}", recv_channel);
        if recv_channel != channel.value {
            return Err(recv_channel);
        }

        // debug print
        let recv_length = unsafe { ptr::read_volatile(&BUFFER[0]) } as usize / size_of::<u32>();
        for i in 0..recv_length {
            let n = unsafe { ptr::read_volatile(&BUFFER[i]) };
            debug!("{:#010x}", n);
        }

        Ok(recv_channel)
    }

    pub fn _test(&self) {
        /*
        //debug prints
        {:#034b}
        {:#010x}
        */

        /*
        // get board revision
        #[rustfmt::skip]
        let msg: [u32; 8] = [
            32,
            0,
            0x00010002, 4, 0, 0,
            0, 0
        ];
        */

        /*
        // enable Status LED
        #[rustfmt::skip]
        let msg: [u32; 8] = [
            32,
            0,
            0x00038041, 8, 0, 130, 1,
            0,
        ];
        */
    }
}

impl MailBox {
    pub const COMPATIBLE: &'static str = "BCM Mailbox";

    /// Create an instance.
    ///
    /// # Safety
    ///
    /// - The user must ensure to provide a correct MMIO start address.
    pub const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            inner: NullLock::new(MailBoxInner::new(mmio_start_addr)),
        }
    }

    pub fn request_framebuffer(&self) -> Option<Display> {
        self.inner.lock(|inner| inner.request_framebuffer())
    }

    // DEBUG
    pub fn _test(&self) {
        self.inner.lock(|inner| inner._test())
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------
use synchronization::interface::Mutex;

impl driver::interface::DeviceDriver for MailBox {
    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }
}
