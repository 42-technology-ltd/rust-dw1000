//! Driver crate for the DW1000 UWB transceiver


#![no_std]

#![deny(missing_docs)]
#![deny(warnings)]


extern crate nrf52_hal;


use nrf52_hal::{
    prelude::*,
    gpio::{
        p0,
        Output,
        PushPull,
    },
    spim,
    Spim,
};


/// Entry point to the DW1000 driver API
pub struct DW1000<SPI> {
    spim       : Spim<SPI>,
    chip_select: p0::P0_Pin<Output<PushPull>>,
}

impl<SPI> DW1000<SPI> where SPI: SpimExt {
    /// Create a new instance of `DW1000`
    ///
    /// Requires the SPI peripheral and the chip select pin that are connected
    /// to the DW1000.
    pub fn new(
        spim       : Spim<SPI>,
        chip_select: p0::P0_Pin<Output<PushPull>>
    )
        -> Self
    {
        DW1000 {
            spim,
            chip_select,
        }
    }

    /// Read from a register
    pub fn read<R: Register + Readable>(&mut self) -> Result<R, spim::Error> {
        let tx_buffer = [make_header::<R>(false)];

        let mut r = R::new();

        self.spim.read(&mut self.chip_select, &tx_buffer, r.buffer())?;

        Ok(r)
    }

    /// Write to a register
    pub fn write<R, F>(&mut self, f: F) -> Result<(), spim::Error>
        where
            R: Register + Writable,
            F: FnOnce(&mut R) -> &mut R,
    {
        let mut r = R::new();
        f(&mut r);
        let tx_buffer = r.buffer();
        tx_buffer[0] = make_header::<R>(true);

        self.spim.write(&mut self.chip_select, &tx_buffer)?;

        Ok(())
    }
}


fn make_header<R: Register>(write: bool) -> u8 {
    ((write as u8) << 7 & 0x80) |
    (0             << 6 & 0x40) |  // no sub-index
    (R::ID              & 0x3f)
}


/// Implemented for all registers
///
/// The DW1000 user manual, section 7.1, specifies what the values of the
/// constant should be for each register.
pub trait Register {
    /// The register index
    const ID:  u8;

    /// The lenght of the register
    const LEN: usize;

    /// Creates an instance of the register
    fn new() -> Self;

    /// Returns a mutable reference to the register's internal buffer
    fn buffer(&mut self) -> &mut [u8];
}

/// Marker trait for registers that can be read
pub trait Readable {}

/// Marker trait for registers that can be written
pub trait Writable {}

macro_rules! impl_register {
    ($($id:expr, $len:expr, $rw:tt, $name:ident; #[$doc:meta])*) => {
        $(
            #[$doc]
            #[allow(non_camel_case_types)]
            pub struct $name([u8; $len + 1]);

            impl Register for $name {
                const ID:  u8    = $id;
                const LEN: usize = $len;

                fn new() -> Self {
                    $name([0; $len + 1])
                }

                fn buffer(&mut self) -> &mut [u8] {
                    &mut self.0
                }
            }

            impl_rw!($rw, $name);
        )*
    }
}

macro_rules! impl_rw {
    (RO, $name:ident) => {
        impl_rw!(@R, $name);
    };
    (RW, $name:ident) => {
        impl_rw!(@R, $name);
        impl_rw!(@W, $name);
    };

    (@R, $name:ident) => {
        impl Readable for $name {}
    };
    (@W, $name:ident) => {
        impl Writable for $name {}
    };
}

impl_register! {
    0x00, 4, RO, DEV_ID; /// Device identifier
    0x01, 8, RW, EUI;    /// Extended Unique Identifier
    0x03, 4, RW, PANADR; /// PAN Identifier and Short Address
}


impl DEV_ID {
    /// Register Identification Tag
    pub fn ridtag(&self) -> u16 {
        ((self.0[4] as u16) << 8) | self.0[3] as u16
    }

    /// Model
    pub fn model(&self) -> u8 {
        self.0[2]
    }

    /// Version
    pub fn ver(&self) -> u8 {
        (self.0[1] & 0xf0) >> 4
    }

    /// Revision
    pub fn rev(&self) -> u8 {
        self.0[1] & 0x0f
    }
}

impl EUI {
    /// Extended Unique Identifier
    pub fn eui(&self) -> u64 {
        ((self.0[8] as u64) << 56) |
            ((self.0[7] as u64) << 48) |
            ((self.0[6] as u64) << 40) |
            ((self.0[5] as u64) << 32) |
            ((self.0[4] as u64) << 24) |
            ((self.0[3] as u64) << 16) |
            ((self.0[2] as u64) <<  8) |
            self.0[1] as u64
    }

    /// Extended Unique Identifier
    pub fn set_eui(&mut self, value: u64) -> &mut Self {
        self.0[8] = ((value & 0xff00000000000000) >> 56) as u8;
        self.0[7] = ((value & 0x00ff000000000000) >> 48) as u8;
        self.0[6] = ((value & 0x0000ff0000000000) >> 40) as u8;
        self.0[5] = ((value & 0x000000ff00000000) >> 32) as u8;
        self.0[4] = ((value & 0x00000000ff000000) >> 24) as u8;
        self.0[3] = ((value & 0x0000000000ff0000) >> 16) as u8;
        self.0[2] = ((value & 0x000000000000ff00) >>  8) as u8;
        self.0[1] = (value & 0x00000000000000ff) as u8;

        self
    }
}

impl PANADR {
     /// Short Address
    pub fn short_addr(&self) -> u16 {
        ((self.0[2] as u16) << 8) | self.0[1] as u16
    }

    /// PAN Identifier
    pub fn pan_id(&self) -> u16 {
        ((self.0[4] as u16) << 8) | self.0[3] as u16
    }

    /// Short Address
    pub fn set_short_addr(mut self, value: u16) -> Self {
        self.0[2] = ((value & 0xff00) >> 8) as u8;
        self.0[1] = (value & 0x00ff) as u8;

        self
    }

    /// PAN Identifier
    pub fn set_pan_id(mut self, value: u16) -> Self {
        self.0[4] = ((value & 0xff00) >> 8) as u8;
        self.0[3] = (value & 0x00ff) as u8;

        self
    }
}


#[cfg(test)]
mod tests {
    use super::{
        Register,
        DEV_ID,
        EUI,
        PANADR,
    };


    #[test]
    fn dev_id_should_provide_access_to_its_fields() {
        let dev_id = DEV_ID([0x00, 0x30, 0x01, 0xca, 0xde]);

        assert_eq!(dev_id.rev()   , 0     );
        assert_eq!(dev_id.ver()   , 3     );
        assert_eq!(dev_id.model() , 1     );
        assert_eq!(dev_id.ridtag(), 0xDECA);
    }

    #[test]
    fn eui_should_provide_access_to_its_field() {
        let eui = EUI([0x00, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);

        assert_eq!(eui.eui(), 0xf0debc9a78563412);

        let mut eui = EUI::new();
        assert_eq!(eui.eui(), 0);
        eui.set_eui(0xf0debc9a78563412);
        assert_eq!(eui.eui(), 0xf0debc9a78563412);
    }

    #[test]
    fn panadr_should_provide_access_to_its_fields() {
        let panadr = PANADR([0x00, 0x01, 0x23, 0x45, 0x67]);

        assert_eq!(panadr.short_addr(), 0x2301);
        assert_eq!(panadr.pan_id(),     0x6745);

        let panadr = PANADR::new();

        assert_eq!(panadr.short_addr(), 0);
        assert_eq!(panadr.pan_id(),     0);

        let panadr = panadr.set_short_addr(0x2301);
        assert_eq!(panadr.short_addr(), 0x2301);

        let panadr = panadr.set_pan_id(0x6745);
        assert_eq!(panadr.pan_id(), 0x6745);
    }
}
