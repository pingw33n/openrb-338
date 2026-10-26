#!/bin/sh
# Builds rbhost.exe (i686 Windows) on the Mac with llvm-mingw.
#
#   tools/rbhost/build.sh [OUT.exe]        default: tools/rbhost/build/rbhost.exe
#
# The toolchain is the pinned llvm-mingw of ~/dev/dcre/common/tools/fetch_llvm_mingw.sh, installed
# in $LLVM_MINGW (default ~/llvm-mingw). -mfpmath=sse keeps rbhost's own double arithmetic (the export
# chunk loop) in IEEE double; the DLL's x87 code runs with the control word rbhost sets (0x027F).
set -e
here=$(cd "$(dirname "$0")" && pwd)
toolchain=${LLVM_MINGW:-$HOME/llvm-mingw}
cc=$toolchain/bin/i686-w64-mingw32-clang
if [ ! -x "$cc" ]; then
    echo "no $cc: run ~/dev/dcre/common/tools/fetch_llvm_mingw.sh (or set LLVM_MINGW)" >&2
    exit 1
fi
out=${1:-$here/build/rbhost.exe}
mkdir -p "$(dirname "$out")"
"$cc" -std=c11 -O2 -Wall -Wextra -Wno-unused-parameter -msse2 -mfpmath=sse -static \
    -o "$out" "$here/rbhost.c"
echo "$out"
