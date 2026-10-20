#!/usr/bin/env python3
"""Validate packaged Android/JVM API and native ABI contents."""
from pathlib import Path
import io
import os
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
abi_machines = {'arm64-v8a': 183, 'armeabi-v7a': 40, 'x86_64': 62}
if os.environ.get('BARK_PR_BUILD') == '1':
    del abi_machines['armeabi-v7a']
aar = ROOT / 'sdk/android/build/outputs/aar/android-release.aar'
with zipfile.ZipFile(aar) as package:
    native = [n for n in package.namelist() if n.startswith('jni/') and n.endswith('.so')]
    assert {n.split('/')[1] for n in native} == set(abi_machines), native
    for name in native:
        data = package.read(name)
        assert data[:4] == b'\x7fELF', name
        assert struct.unpack('<H', data[18:20])[0] == abi_machines[name.split('/')[1]], name
    classes = zipfile.ZipFile(io.BytesIO(package.read('classes.jar')))
    for name in ['Wallet', 'OnchainWallet', 'NotificationHolder', 'BarkError']:
        assert 'io/github/niteshbalusu11/bark/' + name + '.class' in classes.namelist(), name
jars = list((ROOT / 'sdk/jvm/build/libs').glob('bark-bolt-ffi-jvm-*.jar'))
assert jars, 'No versioned JVM JAR'
for jar in jars:
    with zipfile.ZipFile(jar) as package:
        names = package.namelist()
        assert 'io/github/niteshbalusu11/bark/Wallet.class' in names
        assert any(n.endswith(('.dylib', '.so')) for n in names), jar
print(f'AAR has all {len(abi_machines)} expected Android ELF ABIs and public classes; JVM JAR includes API and desktop natives')
