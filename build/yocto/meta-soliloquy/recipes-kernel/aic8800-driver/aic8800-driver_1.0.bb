# BitBake Recipe for AIC8800 WiFi Driver
# SPDX-License-Identifier: Apache-2.0
#
# This recipe builds the AIC8800 WiFi chipset driver for Linux systems.
# Supports AIC8800D, AIC8800DC, and AIC8800DW variants.

SUMMARY = "AIC8800 WiFi Driver for Soliloquy"
DESCRIPTION = "Out-of-tree kernel module for AIC Semiconductor AIC8800 WiFi/BT chipset. \
Supports SDIO interface and multiple chip variants."
HOMEPAGE = "https://github.com/soliloquy-os/soliloquy"
SECTION = "kernel/modules"
LICENSE = "Apache-2.0"
LIC_FILES_CHKSUM = "file://${COMMON_LICENSE_DIR}/Apache-2.0;md5=89aea4e17d99a7cacdbeed46a0096b10"

# Inherit kernel module class
inherit module

# Source files - using local files from Soliloquy tree
SRC_URI = " \
    file://Makefile \
    file://Kconfig \
    file://aic8800_fdrv/ \
    file://aic_load_fw/ \
"

# Required dependencies
DEPENDS = "virtual/kernel"
RDEPENDS:${PN} = "kernel-modules"

# Package version
PV = "1.0"
PR = "r0"

# Module configuration options
EXTRA_OEMAKE = " \
    KERNELDIR=${STAGING_KERNEL_DIR} \
    KERNEL_SRC=${STAGING_KERNEL_DIR} \
    KDIR=${STAGING_KERNEL_DIR} \
    ARCH=${ARCH} \
    CROSS_COMPILE=${TARGET_PREFIX} \
"

# Platform-specific configuration
# These can be overridden in local.conf or machine configuration
AIC_PLATFORM ?= "generic"
AIC_PREALLOC_RX_SKB ?= "y"
AIC_PREALLOC_TXQ ?= "y"
AIC_USE_FW_REQUEST ?= "n"

do_configure:prepend() {
    # Create config header based on platform settings
    cat > ${S}/config.h << EOF
/* Auto-generated AIC8800 configuration */
#ifndef AIC8800_CONFIG_H
#define AIC8800_CONFIG_H

#define CONFIG_PLATFORM_${AIC_PLATFORM} 1
#define CONFIG_PREALLOC_RX_SKB ${AIC_PREALLOC_RX_SKB}
#define CONFIG_PREALLOC_TXQ ${AIC_PREALLOC_TXQ}
#define CONFIG_USE_FW_REQUEST ${AIC_USE_FW_REQUEST}

#endif /* AIC8800_CONFIG_H */
EOF
}

do_compile() {
    # Build the kernel modules
    oe_runmake -C ${STAGING_KERNEL_DIR} M=${S} modules
}

do_install() {
    # Install kernel modules
    install -d ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800
    
    # Install aic_load_fw module
    install -m 0644 ${S}/aic_load_fw/aic_load_fw.ko \
        ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800/
    
    # Install aic8800_fdrv module
    install -m 0644 ${S}/aic8800_fdrv/aic8800_fdrv.ko \
        ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800/
}

# Also create a firmware package
PACKAGES =+ "${PN}-firmware"

# Firmware files location
FIRMWARE_INSTALL_DIR = "${nonarch_base_libdir}/firmware/aic8800"

do_install:append() {
    # Install firmware files
    install -d ${D}${FIRMWARE_INSTALL_DIR}
    
    # Install firmware for AIC8800D80 variant
    if [ -d "${WORKDIR}/firmware/aic8800D80" ]; then
        install -d ${D}${FIRMWARE_INSTALL_DIR}/aic8800D80
        install -m 0644 ${WORKDIR}/firmware/aic8800D80/* \
            ${D}${FIRMWARE_INSTALL_DIR}/aic8800D80/ 2>/dev/null || true
    fi
    
    # Install firmware for AIC8800DC variant
    if [ -d "${WORKDIR}/firmware/aic8800DC" ]; then
        install -d ${D}${FIRMWARE_INSTALL_DIR}/aic8800DC
        install -m 0644 ${WORKDIR}/firmware/aic8800DC/* \
            ${D}${FIRMWARE_INSTALL_DIR}/aic8800DC/ 2>/dev/null || true
    fi
}

FILES:${PN} = " \
    ${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800/*.ko \
"

FILES:${PN}-firmware = " \
    ${FIRMWARE_INSTALL_DIR}/* \
"

# Module autoload configuration
KERNEL_MODULE_AUTOLOAD += "aic_load_fw aic8800_fdrv"

# Module parameters can be set via modprobe.d
CONFFILES:${PN} = "${sysconfdir}/modprobe.d/aic8800.conf"

do_install:append() {
    install -d ${D}${sysconfdir}/modprobe.d
    cat > ${D}${sysconfdir}/modprobe.d/aic8800.conf << EOF
# AIC8800 WiFi Driver Configuration
# Uncomment options as needed

# options aic8800_fdrv power_save=0
# options aic8800_fdrv debug_level=1
EOF
}

# Compatible machines - add your specific machine here
COMPATIBLE_MACHINE = "(qemu|arm64|aarch64|x86_64|rockchip|allwinner|amlogic|.*)"

# Provide generic name for dependencies
PROVIDES = "aic8800-wifi-driver"
RPROVIDES:${PN} = "aic8800-wifi-driver"
