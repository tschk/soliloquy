FROM alpine:3.21

RUN apk add --no-cache bash openrc alpine-baselayout busybox

WORKDIR /work
COPY system/alpine /work/system/alpine

RUN mkdir -p /out/rootfs && \
    mkdir -p /out/rootfs/etc/apk && \
    cp /etc/apk/repositories /out/rootfs/etc/apk/repositories && \
    apk --root /out/rootfs \
      --initdb \
      --update-cache \
      --repositories-file /out/rootfs/etc/apk/repositories \
      --keys-dir /etc/apk/keys \
      add alpine-baselayout busybox openrc && \
    xargs apk --root /out/rootfs \
      --update-cache \
      --repositories-file /out/rootfs/etc/apk/repositories \
      --keys-dir /etc/apk/keys \
      add < /work/system/alpine/packages-v0.txt && \
    /work/system/alpine/scripts/configure-rootfs.sh /out/rootfs && \
    tar --exclude='rootfs/dev/*' --exclude='rootfs/proc/*' --exclude='rootfs/sys/*' -czf /out/rootfs.tar.gz -C /out rootfs

CMD ["/bin/sh", "-lc", "echo rootfs prepared"]
