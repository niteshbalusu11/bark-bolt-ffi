#!/usr/bin/env python3
"""Run one trusted RegTest Actions job on this Mac, without installing a service."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = 'niteshbalusu11/bark-bolt-ffi'
ARCHIVE = 'https://github.com/actions/runner/releases/download/v2.338.0/actions-runner-osx-arm64-2.338.0.tar.gz'
CHECKSUM = 'df4cebda25c86a886ed204e49fee63f5c2e7cec5f447b5c98440a826bbdf9df2'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ios-device', required=True)
    parser.add_argument('--android-device', default='emulator-5554')
    options = parser.parse_args()
    environment = os.environ | {
        'PATH': '/opt/homebrew/bin:' + str(Path.home() / 'Library/Android/sdk/platform-tools') + ':' + os.environ['PATH'],
        'JAVA_HOME': '/opt/homebrew/opt/openjdk@17',
        'ANDROID_HOME': str(Path.home() / 'Library/Android/sdk'),
        'IOS_DEVICE': options.ios_device,
        'ANDROID_SERIAL': options.android_device,
        'CARGO_TARGET_DIR': str(ROOT / 'target'),
    }
    gh = shutil.which('gh', path=environment['PATH'])
    if not gh:
        raise RuntimeError('GitHub CLI is required with repository admin authentication')
    with tempfile.TemporaryDirectory(prefix='bark-bolt-ci-runner-') as temporary:
        directory = Path(temporary)
        archive = directory / 'runner.tar.gz'
        urllib.request.urlretrieve(ARCHIVE, archive)
        digest = hashlib.sha256()
        with archive.open('rb') as source:
            for part in iter(lambda: source.read(1024 * 1024), b''):
                digest.update(part)
        assert digest.hexdigest() == CHECKSUM, 'Runner archive checksum mismatch'
        with tarfile.open(archive) as source:
            for member in source.getmembers():
                target = (directory / member.name).resolve()
                if directory.resolve() not in target.parents and target != directory.resolve():
                    raise RuntimeError('Runner archive contains an unsafe path')
                if member.issym() or member.islnk():
                    link = (target.parent / member.linkname).resolve()
                    if directory.resolve() not in link.parents:
                        raise RuntimeError('Runner archive contains an unsafe symlink')
            source.extractall(directory)
        token = json.loads(subprocess.check_output([gh, 'api', '--method', 'POST',
            f'repos/{REPOSITORY}/actions/runners/registration-token'], env=environment, text=True))['token']
        name = 'bark-bolt-ephemeral-' + directory.name.removeprefix('bark-bolt-ci-runner-')
        configured = subprocess.run([str(directory / 'config.sh'), '--unattended', '--ephemeral',
            '--url', 'https://github.com/' + REPOSITORY, '--token', token,
            '--name', name, '--labels', 'bark-bolt-regtest', '--work', '_work'],
            cwd=directory, env=environment, text=True, capture_output=True, timeout=120)
        if configured.returncode:
            raise RuntimeError((configured.stdout + configured.stderr).replace(token, '<redacted>'))
        print(f'Ephemeral runner {name} is ready for one job in {REPOSITORY}.', flush=True)
        try:
            subprocess.run([str(directory / 'run.sh'), '--once'], cwd=directory,
                           env=environment, check=True, timeout=7200)
        finally:
            # Ephemeral runners unregister themselves after the job. Remove a
            # registration left by an interrupted listener before deleting it.
            runners = json.loads(subprocess.check_output([gh, 'api', f'repos/{REPOSITORY}/actions/runners'],
                                env=environment, text=True))['runners']
            for runner in runners:
                if runner['name'] == name:
                    subprocess.run([gh, 'api', '--method', 'DELETE',
                                    f"repos/{REPOSITORY}/actions/runners/{runner['id']}"], env=environment, check=True)


if __name__ == '__main__':
    main()
