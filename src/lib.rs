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

    /// Read a register
    pub fn read<R: Register>(&mut self) -> Result<R, spim::Error> {
        let header =
            (0     & 0x80) |  // read
            (0     & 0x40) |  // no sub-index
            (R::ID & 0x3f);   // index of the register
        let tx_buffer = [header];

        let mut r = R::new();

        self.spim.read(&mut self.chip_select, &tx_buffer, r.rx_buffer())?;

        Ok(r)
    }
}


/// Implemented for all registers
///
/// This trait is for internal use only. Users of this library should never need
/// to implement this trait, nor use its associated items.
///
/// The DW1000 user manual, section 7.1, specifies what the values of those
/// constant should be for each register.
pub trait Register {
    /// The register ID
    const ID:  u8;

    /// The lenght of the register
    const LEN: usize;

    /// Creates an instance of the register
    fn new() -> Self;

    /// Returns a mutable reference to the register's internal buffer
    ///
    /// SPI is a synchronous interface, which means a bytes is received for
    /// every byte that is sent, even though the bytes we receive while sending
    /// something end up being ignored. Still, we need room for those bytes in
    /// the buffer, so the length of the buffer must be equal to the length of
    /// the register plus the length of the transaction header.
    fn rx_buffer(&mut self) -> &mut [u8];
}

macro_rules! impl_register {
    ($($id:expr, $len:expr, $name:ident; #[$doc:meta])*) => {
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

                fn rx_buffer(&mut self) -> &mut [u8] {
                    &mut self.0
                }
            }
        )*
    }
}

impl_register! {
    0x00, 4, DEV_ID; /// Device identifier
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


#[cfg(test)]
mod tests {
    use super::DEV_ID;


    #[test]
    fn dev_id_should_provide_access_to_its_fields() {
        let dev_id = DEV_ID([0x00, 0x30, 0x01, 0xca, 0xde]);

        assert_eq!(dev_id.rev()   , 0     );
        assert_eq!(dev_id.ver()   , 3     );
        assert_eq!(dev_id.model() , 1     );
        assert_eq!(dev_id.ridtag(), 0xDECA);
    }
}
