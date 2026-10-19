#!/usr/bin/env python3
"""Stage or verify the versioned, optimized SwiftPM SDK."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SDK = ROOT / 'sdk/swift'
TARGETS = {
    ('ios', None, 'arm64'): 'aarch64-apple-ios',
    ('ios', 'simulator', 'arm64'): 'aarch64-apple-ios-sim',
    ('ios', 'simulator', 'x86_64'): 'x86_64-apple-ios',
    ('macos', None, 'arm64'): 'aarch64-apple-darwin',
    ('macos', None, 'x86_64'): 'x86_64-apple-darwin',
}


def checksum(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for part in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(part)
    return digest.hexdigest()


def source_digest():
    digest = hashlib.sha256()
    files = sorted((ROOT / 'src').rglob('*.rs')) + [ROOT / name for name in
        ['Cargo.toml', 'Cargo.lock', 'boltffi.toml', 'rust-toolchain.toml', 'sdk/swift-support/Concurrency.swift']]
    for path in files:
        digest.update(str(path.relative_to(ROOT)).encode() + b'\0' + path.read_bytes() + b'\0')
    return digest.hexdigest()


def artifacts():
    return {str(p.relative_to(SDK)): checksum(p) for p in sorted(SDK.rglob('*'))
            if p.is_file() and p.name != 'provenance.json' and p.name != '.DS_Store'}


def stage():
    generated = ROOT / 'dist/apple'
    framework = generated / 'BarkBoltFFI.xcframework'
    info = plistlib.loads((framework / 'Info.plist').read_bytes())
    target_dir = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    seen = set()
    with tempfile.TemporaryDirectory(prefix='bark-swift-check-') as directory:
        for library in info['AvailableLibraries']:
            binary = framework / library['LibraryIdentifier'] / library['LibraryPath']
            architectures = library['SupportedArchitectures']
            for arch in architectures:
                key = (library['SupportedPlatform'], library.get('SupportedPlatformVariant'), arch)
                assert key in TARGETS, f'Unexpected Apple slice: {key}'
                thin = binary
                if len(architectures) > 1:
                    thin = Path(directory) / ('-'.join(str(v) for v in key) + '.a')
                    subprocess.run(['xcrun', 'lipo', str(binary), '-thin', arch, '-output', str(thin)], check=True)
                expected = target_dir / TARGETS[key] / 'release/libbark_bolt_ffi.a'
                assert checksum(thin) == checksum(expected), f'{key} is not the current release build'
                seen.add(key)
        assert seen == set(TARGETS), f'Missing Apple slices: {set(TARGETS) - seen}'
    SDK.mkdir(parents=True, exist_ok=True)
    for name in ['FFI', 'Sources']:
        destination = SDK / name
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(generated / name, destination)
    archive = ROOT / 'dist/BarkBoltFFI.xcframework.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as output:
        for file in sorted(framework.rglob('*')):
            if file.is_file():
                output.write(file, file.relative_to(framework.parent))
    archive_hash = checksum(archive)
    package = (generated / 'Package.swift').read_text().replace('path: "', 'path: "sdk/swift/')
    package = package.replace('path: "sdk/swift/BarkBoltFFI.xcframework"',
        'url: "https://github.com/smolcars/bark-bolt-ffi/releases/download/v0.1.0/BarkBoltFFI.xcframework.zip", checksum: "' + archive_hash + '"')
    (ROOT / 'Package.swift').write_text(package + '\n')
    (SDK / 'provenance.json').write_text(json.dumps({
        'version': '0.1.0', 'bark': '0.7.1-dev', 'bark_revision': 'bfbe6b7c3088a09dd4e81c4ac24cdc763a627a8c', 'boltffi': '0.31.1',
        'rust': '1.90.0', 'profile': 'release', 'archive_sha256': archive_hash, 'source_sha256': source_digest(),
        'files': artifacts(), 'package_sha256': checksum(ROOT / 'Package.swift'),
    }, indent=2, sort_keys=True) + '\n')
    print(f'Staged release SwiftPM manifest and generated sources; archive: {archive.stat().st_size} bytes')


def verify():
    metadata = json.loads((SDK / 'provenance.json').read_text())
    assert metadata['source_sha256'] == source_digest(), 'Swift SDK is stale; rebuild release SDK and stage it'
    assert metadata['files'] == artifacts(), 'Swift SDK files changed or are missing'
    assert metadata['package_sha256'] == checksum(ROOT / 'Package.swift'), 'Root SwiftPM manifest changed'
    assert metadata['profile'] == 'release'
    assert metadata['archive_sha256'] in (ROOT / 'Package.swift').read_text()
    print('Swift SDK source, artifact and manifest checksums match')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage-release', action='store_true')
    options = parser.parse_args()
    if options.stage_release:
        stage()
    verify()
