# meta-soliloquy

Yocto/OpenEmbedded layer for Soliloquy platform device drivers.

## Description

This layer provides BitBake recipes for building Soliloquy platform drivers as out-of-tree Linux kernel modules. It includes support for:

- **aic8800-driver**: WiFi driver for AIC Semiconductor AIC8800 chipset (SDIO interface)
- **soliloquy-drivers**: Complete collection of Soliloquy platform drivers

## Layer Dependencies

- openembedded-core (or poky)
- meta-kernel (for kernel module base class)

## Adding the Layer

Add the layer to your `bblayers.conf`:

```bitbake
BBLAYERS += "${BSPDIR}/sources/meta-soliloquy"
```

Or use `bitbake-layers`:

```bash
bitbake-layers add-layer meta-soliloquy
```

## Available Recipes

### aic8800-driver

Standalone recipe for the AIC8800 WiFi driver only.

```bash
bitbake aic8800-driver
```

### soliloquy-drivers

Complete collection of all Soliloquy platform drivers (WiFi, GPIO, GPU).

```bash
bitbake soliloquy-drivers
```

Sub-packages:
- `soliloquy-drivers-wifi` - WiFi driver modules
- `soliloquy-drivers-gpio` - GPIO driver modules  
- `soliloquy-drivers-gpu` - GPU driver modules
- `soliloquy-drivers-firmware` - Firmware files

## Preparing Sources

Before building, run the preparation script to copy driver sources to the recipe files directory:

```bash
./recipes-kernel/soliloquy-drivers/prepare_yocto_sources.sh
```

## Configuration Variables

You can customize the build by setting variables in `local.conf`:

```bitbake
# Platform selection (generic, rockchip, allwinner, amlogic)
AIC_PLATFORM = "generic"

# Enable/disable features
AIC_PREALLOC_RX_SKB = "y"
AIC_PREALLOC_TXQ = "y"
AIC_USE_FW_REQUEST = "n"
```

## Installation

When included in your image, the drivers will be installed to:

- Kernel modules: `/lib/modules/<kernel-version>/kernel/drivers/net/wireless/aic8800/`
- Firmware: `/lib/firmware/aic8800/`
- Configuration: `/etc/modprobe.d/soliloquy-drivers.conf`

## Module Loading

By default, modules are configured for autoloading. You can manually load them:

```bash
modprobe aic_load_fw
modprobe aic8800_fdrv
```

## License

Apache-2.0

## Maintainer

Soliloquy Project
