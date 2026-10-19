#!/usr/bin/env python3
"""Bounded readiness checks for the simulator/emulator selected by CI or a developer."""
import argparse
import os
import shutil
import subprocess
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ios-device', default=os.environ.get('IOS_DEVICE'))
parser.add_argument('--android-device', default=os.environ.get('ANDROID_SERIAL'))
options = parser.parse_args()
assert options.ios_device or options.android_device, 'Select an iOS simulator or Android emulator'
if options.ios_device:
    subprocess.run(['xcrun', 'simctl', 'bootstatus', options.ios_device, '-b'], check=True, timeout=180)
    print('iOS simulator ready', flush=True)
if options.android_device:
    adb = shutil.which('adb') or str(Path.home() / 'Library/Android/sdk/platform-tools/adb')
    end = time.monotonic() + 180
    while time.monotonic() < end:
        result = subprocess.run([adb, '-s', options.android_device, 'shell', 'getprop', 'sys.boot_completed'],
                                text=True, capture_output=True, timeout=5)
        if result.returncode == 0 and result.stdout.strip() == '1':
            print('Android emulator ready', flush=True)
            break
        time.sleep(1)
    else:
        raise SystemExit('Android emulator did not finish booting in 180s')
