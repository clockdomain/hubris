# Build
argo xtask dist app/ast1060-i2c-scaffold/app.toml 

# Local clone of openprot 

This folder has a local clone where the mock i2c hardware is avaiable. 
You can use to speed up your inspection , but do not update Cargo.toml with it.
~/rusty1968/prot-i2c/oprot

# Architectural mismatch
┌─────────────────────────────────────────┐
│    Controller Instance (This Document)  │  ← I2cHardware SHOULD be here
│  • I2cHardwareCore                     │    (as I2cControllerCore, etc.)
│  • I2cMaster                           │
│  • I2cSlaveCore + I2cSlaveBuffer       │
├─────────────────────────────────────────┤
│       Platform Driver Layer            │  ← I2cHardware is ALSO here
│  (drv-stm32xx-i2c, drv-ast1060-i2c)   │    (should be register access only)
└─────────────────────────────────────────┘