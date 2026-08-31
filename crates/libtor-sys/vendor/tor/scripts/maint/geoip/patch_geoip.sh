#!/bin/sh

set -e

CFGDIR=$(pwd)
RULES="${CFGDIR}/geoip-overrides.sed"

if [ -f "${RULES}" ]; then
    sed -i -f "${RULES}" "${CFGDIR}/geoip"
    sed -i -f "${RULES}" "${CFGDIR}/geoip6"
fi
