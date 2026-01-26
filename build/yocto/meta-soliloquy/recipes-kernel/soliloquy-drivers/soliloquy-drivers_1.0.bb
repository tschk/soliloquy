# BitBake Recipe for All Soliloquy Drivers
# SPDX-License-Identifier: Apache-2.0
#
# This recipe builds all Soliloquy device drivers for Linux systems.
# Includes WiFi (AIC8800), GPIO, GPU, and other platform drivers.

SUMMARY = "Soliloquy Device Drivers Collection"
DESCRIPTION = "Complete collection of device drivers for Soliloquy platform including \
WiFi (AIC8800), GPIO, GPU, and generic platform drivers. Built as out-of-tree kernel modules."
HOMEPAGE = "https://github.com/soliloquy-os/soliloquy"
SECTION = "kernel/modules"
LICENSE = "Apache-2.0"
LIC_FILES_CHKSUM = "file://${COMMON_LICENSE_DIR}/Apache-2.0;md5=89aea4e17d99a7cacdbeed46a0096b10"

inherit module

# Fetch from local Soliloquy source tree
# Adjust SOLILOQUY_SRC to your local path or use a git repository
SOLILOQUY_SRC ?= "${TOPDIR}/../soliloquy"

SRC_URI = " \
    file://drivers \
"

S = "${WORKDIR}/drivers"

DEPENDS = "virtual/kernel"
RDEPENDS:${PN} = "kernel-modules"

PV = "1.0"
PR = "r0"

# Build configuration
EXTRA_OEMAKE = " \
    KERNELDIR=${STAGING_KERNEL_DIR} \
    KERNEL_SRC=${STAGING_KERNEL_DIR} \
    ARCH=${ARCH} \
    CROSS_COMPILE=${TARGET_PREFIX} \
"

# Sub-packages for different driver categories
PACKAGES =+ " \
    ${PN}-wifi \
    ${PN}-gpio \
    ${PN}-gpu \
    ${PN}-firmware \
"

do_compile() {
    # Build WiFi driver (AIC8800)
    if [ -d "${S}/wifi/aic8800/linux_reference" ]; then
        bbnote "Building AIC8800 WiFi driver..."
        oe_runmake -C ${STAGING_KERNEL_DIR} \
            M=${S}/wifi/aic8800/linux_reference \
            modules
    fi
    
    # Build GPIO drivers if present
    if [ -d "${S}/gpio" ] && [ -f "${S}/gpio/Makefile" ]; then
        bbnote "Building GPIO drivers..."
        oe_runmake -C ${STAGING_KERNEL_DIR} \
            M=${S}/gpio \
            modules
    fi
    
    # Build GPU drivers if present
    if [ -d "${S}/gpu" ] && [ -f "${S}/gpu/Makefile" ]; then
        bbnote "Building GPU drivers..."
        oe_runmake -C ${STAGING_KERNEL_DIR} \
            M=${S}/gpu \
            modules
    fi
}

do_install() {
    # Create installation directories
    install -d ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800
    install -d ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpio
    install -d ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpu
    install -d ${D}${nonarch_base_libdir}/firmware/aic8800
    
    # Install WiFi modules
    if [ -d "${S}/wifi/aic8800/linux_reference" ]; then
        find ${S}/wifi/aic8800/linux_reference -name "*.ko" -exec \
            install -m 0644 {} \
            ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800/ \;
    fi
    
    # Install GPIO modules
    if [ -d "${S}/gpio" ]; then
        find ${S}/gpio -name "*.ko" -exec \
            install -m 0644 {} \
            ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpio/ \;
    fi
    
    # Install GPU modules
    if [ -d "${S}/gpu" ]; then
        find ${S}/gpu -name "*.ko" -exec \
            install -m 0644 {} \
            ${D}${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpu/ \;
    fi
    
    # Install firmware
    if [ -d "${S}/wifi/aic8800/firmware" ]; then
        cp -r ${S}/wifi/aic8800/firmware/* ${D}${nonarch_base_libdir}/firmware/aic8800/
    fi
    
    # Install modprobe configuration
    install -d ${D}${sysconfdir}/modprobe.d
    cat > ${D}${sysconfdir}/modprobe.d/soliloquy-drivers.conf << EOF
# Soliloquy Platform Driver Configuration

# AIC8800 WiFi Driver
# options aic8800_fdrv power_save=0
# options aic8800_fdrv debug_level=0

# GPIO Driver
# options soliloquy_gpio debug=0

# GPU Driver  
# options soliloquy_gpu memory_pool=64
EOF
}

# Package file assignments
FILES:${PN}-wifi = " \
    ${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/net/wireless/aic8800/*.ko \
"

FILES:${PN}-gpio = " \
    ${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpio/*.ko \
"

FILES:${PN}-gpu = " \
    ${nonarch_base_libdir}/modules/${KERNEL_VERSION}/kernel/drivers/gpu/*.ko \
"

FILES:${PN}-firmware = " \
    ${nonarch_base_libdir}/firmware/aic8800/* \
"

FILES:${PN} = " \
    ${sysconfdir}/modprobe.d/soliloquy-drivers.conf \
"

# Runtime dependencies
RDEPENDS:${PN}-wifi = "${PN}-firmware"

# Module autoload
KERNEL_MODULE_AUTOLOAD += "aic_load_fw aic8800_fdrv"

# Allow empty packages (in case some drivers aren't present)
ALLOW_EMPTY:${PN} = "1"
ALLOW_EMPTY:${PN}-gpio = "1"
ALLOW_EMPTY:${PN}-gpu = "1"

COMPATIBLE_MACHINE = "(.*)"

PROVIDES = "soliloquy-drivers"
