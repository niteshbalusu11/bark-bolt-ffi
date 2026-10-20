#!/usr/bin/env python3
"""Require every LOAD segment in every AAR shared library to support 16 KB pages."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def load_alignments(output):
    alignments = []
    for line in output.splitlines():
        fields = line.split()
        if fields and fields[0] == 'LOAD':
            alignment = int(fields[-1], 16)
            if alignment < 0x4000:
                raise ValueError(f'LOAD Align {alignment:#x} is below 0x4000: {line.strip()}')
            alignments.append(alignment)
    if not alignments:
        raise ValueError('No LOAD segments found in llvm-readelf -lW output')
    return alignments


def find_readelf():
    executable = os.environ.get('LLVM_READELF') or shutil.which('llvm-readelf')
    if executable:
        return executable
    ndk = os.environ.get('ANDROID_NDK_HOME') or os.environ.get('ANDROID_NDK_ROOT')
    sdk = Path(os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT')
               or (Path.home() / 'Library/Android/sdk' if os.uname().sysname == 'Darwin'
                   else Path.home() / 'Android/Sdk'))
    roots = [Path(ndk)] if ndk else [sdk / 'ndk/28.2.13676358']
    for root in roots:
        matches = sorted(root.glob('toolchains/llvm/prebuilt/*/bin/llvm-readelf'))
        if matches:
            return str(matches[0])
    raise RuntimeError('llvm-readelf unavailable; set LLVM_READELF or install NDK 28.2.13676358')


def check_aar(aar, readelf):
    with zipfile.ZipFile(aar) as package, tempfile.TemporaryDirectory(prefix='bark-elf-') as directory:
        libraries = [n for n in package.namelist() if n.endswith('.so')]
        if not libraries:
            raise ValueError(f'{aar}: no shared libraries')
        for index, name in enumerate(libraries):
            # Avoid extracting archive paths supplied by an external release.
            binary = Path(directory) / f'{index}.so'
            binary.write_bytes(package.read(name))
            output = subprocess.check_output([readelf, '-lW', str(binary)], text=True)
            try:
                alignments = load_alignments(output)
            except ValueError as error:
                raise ValueError(f'{name}: {error}') from error
            print(f'{name}: {len(alignments)} LOAD segments, minimum Align {min(alignments):#x}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('aar', nargs='?', type=Path,
                        default=ROOT / 'sdk/android/build/outputs/aar/android-release.aar')
    parser.add_argument('--readelf', default=None)
    options = parser.parse_args()
    check_aar(options.aar, options.readelf or find_readelf())
