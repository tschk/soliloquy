# Mali-G57 GPU Driver

This driver provides support for the ARM Mali-G57 GPU (Valhall architecture) for the Allwinner A527 SoC.

## Architecture

The Mali-G57 is part of ARM's Valhall GPU architecture family. This driver scaffold lays the foundation for full GPU support.

## Files

- `mali_g57.h` - Main driver header with MaliG57 device class definition
- `mali_g57.cc` - Driver implementation with initialization and lifecycle management
- `registers.h` - Hardware register definitions for job manager, MMU, and GPU control

## Register Map

The driver defines register offsets for three main subsystems:

### GPU Control (Base: 0x00000000)
- GPU ID, version, and status registers
- Interrupt control (raw status, clear, mask)
- Power management commands

### Job Manager (Base: 0x00000000)
- Job submission and control registers
- Job interrupt management

### MMU (Base: 0x00002000)
- Address space command and status
- Page table configuration
- Fault handling registers

## Hardware Configuration

- **Base Address**: 0x01800000 (Allwinner A527 SoC)
- **MMIO Size**: 64KB
- **Vendor ID**: 0x13B5 (ARM)
- **Device ID**: 0x0B57 (Mali-G57)

## Future Work

This scaffold provides the basic structure. Future development will add:

- MMIO resource acquisition and mapping
- Clock and power management integration
- Job submission and scheduling
- Memory management unit (MMU) configuration
- Interrupt handling
- OpenGL ES / Vulkan rendering support
