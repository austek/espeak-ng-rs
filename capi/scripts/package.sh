#!/bin/bash
# Install the C library under PREFIX as a drop-in libespeak-ng:
#   lib/libespeak-ng.so.1, lib/libespeak-ng.so, lib/libespeak-ng.a,
#   lib/pkgconfig/espeak-ng.pc, include/espeak-ng/*.h, include/espeak/speak_lib.h
#
# usage: capi/scripts/package.sh <prefix> [<cargo profile dir, default target/release>]
# Build first with: cargo build --release -p espeak-ng-capi
set -eu

prefix=${1:?usage: package.sh <prefix> [profile-dir]}
here=$(cd "$(dirname "$0")/.." && pwd)
profile=${2:-"$here/../target/release"}
version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$here/Cargo.toml" | head -n 1)

mkdir -p "$prefix/lib/pkgconfig" "$prefix/include/espeak-ng" "$prefix/include/espeak"
cp "$profile/libespeak_ng_capi.so" "$prefix/lib/libespeak-ng.so.1"
ln -sf libespeak-ng.so.1 "$prefix/lib/libespeak-ng.so"
cp "$profile/libespeak_ng_capi.a" "$prefix/lib/libespeak-ng.a"
cp "$here"/include/espeak-ng/*.h "$prefix/include/espeak-ng/"
cp "$here/include/espeak/speak_lib.h" "$prefix/include/espeak/speak_lib.h"

while IFS= read -r line; do
  line=${line//@PREFIX@/$prefix}
  line=${line//@VERSION@/$version}
  printf '%s\n' "$line"
done < "$here/espeak-ng.pc.in" > "$prefix/lib/pkgconfig/espeak-ng.pc"
echo "installed libespeak-ng $version to $prefix"
