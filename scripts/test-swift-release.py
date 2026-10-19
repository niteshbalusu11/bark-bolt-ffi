#!/usr/bin/env python3
"""Run Swift contracts from a clean consumer of an exact Git revision."""
import argparse
import re
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--revision', default=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip())
options = parser.parse_args()
assert re.fullmatch(r"[0-9a-f]{40}", options.revision), "Pass a full Git commit SHA"
with tempfile.TemporaryDirectory(prefix='bark-swift-consumer-') as directory:
    consumer = Path(directory)
    (consumer / 'Package.swift').write_text('''// swift-tools-version:5.9
import PackageDescription
let package = Package(name: "ReleaseConsumer", platforms: [.macOS(.v13)],
    dependencies: [.package(url: "https://github.com/smolcars/bark-bolt-ffi.git", revision: "%s")],
    targets: [.testTarget(name: "ReleaseConsumerTests", dependencies: [
        .product(name: "BarkBoltFFI", package: "bark-bolt-ffi")], path: "Tests")])
''' % options.revision)
    shutil.copytree(ROOT / 'tests/swift/Tests', consumer / 'Tests')
    subprocess.run(['swift', 'test', '--package-path', str(consumer)], check=True, timeout=600)
