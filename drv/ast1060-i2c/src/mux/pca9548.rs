//! PCA9548 8-channel I2C multiplexer driver

use super::I2cMuxDriver;
use crate::*;

/// PCA9548 8-channel mux (3-bit addressing)
pub struct Pca9548;

impl I2cMuxDriver for Pca9548 {
    fn configure(
        &self,
        _mux: &I2cMux<'_>,
        _i2c: &mut Ast1060I2c<'_>,
    ) -> Result<(), I2cError> {
        // PCA9548 doesn't need special configuration
        Ok(())
    }

    fn set_segment(
        &self,
        mux: &I2cMux<'_>,
        i2c: &mut Ast1060I2c<'_>,
        segment: drv_i2c_api::Segment,
    ) -> Result<(), I2cError> {
        let idx = segment.to_index();
        if idx >= 8 {
            return Err(I2cError::Invalid);
        }

        // PCA9548 uses one-hot encoding: bit N selects channel N
        let control_byte = 1u8 << idx;

        i2c.write(mux.address, &[control_byte])
    }

    fn reset(
        &self,
        mux: &I2cMux<'_>,
        i2c: &mut Ast1060I2c<'_>,
    ) -> Result<(), I2cError> {
        // Write 0 to disable all channels
        i2c.write(mux.address, &[0])
    }
}
