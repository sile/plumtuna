#!/bin/bash

set -eux

NAME=$1

rm -f /tmp/${NAME}.stats
watch -p -n 5 'echo "=== `date` ===" >> /tmp/'${NAME}'.stats; docker stats --no-stream | tee -a /tmp/'${NAME}'.stats'
