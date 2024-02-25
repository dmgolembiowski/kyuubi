#!/bin/bash
# Using docker-compose syntax with version '3.2',
# you have the ability to mount external disks outside
# of the root file system (e.g. /etc/fstab entries, sshfs mounts, etc.)
# provided they can be a BIND mount within the context of the working
# directory of the invocation of `docker-compose up`.
#
# Therefore, to expose your huge HDDs and fast SSD/NVMEs you need only
# `mount --rbind` them with `sudo -EH this_script.sh` privileges beforehand.
# Note that the environment variables described inside here intentionally
# correspond to the environment variables set out in /.env-example

# Strictly observe that you only target a guaranteed solid-state
# mount. You potentially risk serious damage and data loss if you force parallel
# writing to an HDD.
export SSD_EXTERNAL_MOUNT_DIR=
export HDD_OR_SOLID_STATE_DIR=

mount --rbind  "$SSD_EXTERNAL_MOUNT_DIR" "$(echo $SSD_PATH_DEV)"
mount --rbind  "$HDD_OR_SOLID_STATE_DIR" "$(echo $HDD_PATH_DEV)"
