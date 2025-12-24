# GDB Test Automation for AST1060 I2C Example
# This script monitors I2C initialization and dumps hardware registers

echo \n=== AST1060 I2C Example Test Automation ===\n

# Exception handlers
break HardFault
commands
  echo \n*** HARD FAULT EXCEPTION ***\n
  info registers
  backtrace
  quit
end

break MemManage
commands
  echo \n*** MEM MANAGE FAULT ***\n
  info registers
  backtrace
  quit
end

break BusFault
commands
  echo \n*** BUS FAULT ***\n
  info registers
  backtrace
  quit
end

break UsageFault
commands
  echo \n*** USAGE FAULT ***\n
  info registers
  backtrace
  quit
end

# I2C initialization breakpoints
echo Setting breakpoints on I2C initialization...\n

# Break at pinmux configuration
break configure_i2c_pins
commands
  echo \n=== Configuring I2C2 Pins ===\n
  printf "Controller: %d\n", $r0
  
  # Dump SCU registers before pinmux
  echo \nSCU Registers (before pinmux):\n
  printf "  SCU414 (I2C0-1 pins): 0x%08x\n", *(unsigned int*)0x7e6e2414
  printf "  SCU418 (I2C2-9 pins): 0x%08x\n", *(unsigned int*)0x7e6e2418
  
  continue
end

# Break after pinmux to see changes
break configure_i2c_pins
commands
  echo
  finish
  echo \nSCU Registers (after pinmux):\n
  printf "  SCU414: 0x%08x\n", *(unsigned int*)0x7e6e2414
  printf "  SCU418: 0x%08x\n", *(unsigned int*)0x7e6e2418
  echo   Expected SCU418 bit[0:1] = 1 for I2C2 SCL/SDA\n
  
  continue
end

# Break at global I2C init
break init_i2c_global
commands
  echo \n=== Initializing I2C Global Registers ===\n
  
  # Dump SCU050/054 (reset registers)
  echo SCU Reset Registers:\n
  printf "  SCU050 (assert):   0x%08x\n", *(unsigned int*)0x7e6e2050
  printf "  SCU054 (deassert): 0x%08x\n", *(unsigned int*)0x7e6e2054
  
  continue
end

# Break after global init to see I2CGLOBAL settings
break init_i2c_global
commands
  echo
  finish
  
  echo \nI2C Global Registers (0x7e7b0000):\n
  printf "  I2CG00:  0x%08x\n", *(unsigned int*)0x7e7b0000
  printf "  I2CG04:  0x%08x\n", *(unsigned int*)0x7e7b0004
  printf "  I2CG08:  0x%08x\n", *(unsigned int*)0x7e7b0008
  printf "  I2CG0C:  0x%08x (base clk dividers)\n", *(unsigned int*)0x7e7b000c
  printf "  I2CG10:  0x%08x\n", *(unsigned int*)0x7e7b0010
  
  continue
end

# Break at controller init_hardware
break Ast1060I2c::init_hardware
commands
  echo \n=== I2C Controller init_hardware ===\n
  printf "Config address: 0x%08x\n", $r1
  
  # I2C2 controller registers at 0x7e7b0180
  echo \nI2C2 Controller Registers (0x7e7b0180) - BEFORE init:\n
  printf "  I2CC00 (Func Ctrl):    0x%08x\n", *(unsigned int*)0x7e7b0180
  printf "  I2CC04 (Clk/AC Ctrl):  0x%08x\n", *(unsigned int*)0x7e7b0184
  printf "  I2CC08 (Timing):       0x%08x\n", *(unsigned int*)0x7e7b0188
  printf "  I2CC0C (Timing):       0x%08x\n", *(unsigned int*)0x7e7b018c
  printf "  I2CM10 (Master Int):   0x%08x\n", *(unsigned int*)0x7e7b01d0
  printf "  I2CM14 (Master Stat):  0x%08x\n", *(unsigned int*)0x7e7b01d4
  
  continue
end

# Break after init_hardware
break Ast1060I2c::init_hardware
commands
  echo
  finish
  
  echo \nI2C2 Controller Registers - AFTER init:\n
  printf "  I2CC00 (Func Ctrl):    0x%08x\n", *(unsigned int*)0x7e7b0180
  printf "  I2CC04 (Clk/AC Ctrl):  0x%08x\n", *(unsigned int*)0x7e7b0184
  printf "  I2CC08 (Timing):       0x%08x\n", *(unsigned int*)0x7e7b0188
  printf "  I2CC0C (Timing):       0x%08x\n", *(unsigned int*)0x7e7b018c
  printf "  I2CM10 (Master Int):   0x%08x\n", *(unsigned int*)0x7e7b01d0
  printf "  I2CM14 (Master Stat):  0x%08x\n", *(unsigned int*)0x7e7b01d4
  
  echo \nI2C2 Buffer Registers (0x7e7b0c40):\n
  printf "  Buffer base: 0x7e7b0c40\n"
  
  echo \n=== I2C Init Complete ===\n
  
  continue
end

# Break at main I2C client task
break task_i2c_client::main
commands
  echo \n=== I2C Client Task Started ===\n
  echo Waiting for UART output...\n
  continue
end

# Start execution
echo \nStarting firmware execution...\n
echo Monitoring I2C initialization sequence...\n
echo \n
continue

# If we get here, app ran without hitting breakpoints
echo \n=== Test completed ===\n
quit
