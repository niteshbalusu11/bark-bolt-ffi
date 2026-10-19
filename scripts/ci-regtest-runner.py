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
import uuid

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = 'smolcars/bark-bolt-ffi'
ARCHIVE = 'https://github.com/actions/runner/releases/download/v2.338.0/actions-runner-osx-arm64-2.338.0.tar.gz'
CHECKSUM = 'df4cebda25c86a886ed204e49fee63f5c2e7cec5f447b5c98440a826bbdf9df2'


def child_environment(ios_device, android_device, parent):
    home = Path(parent['HOME'])
    environment = {key: parent[key] for key in ['HOME', 'USER', 'LOGNAME', 'TMPDIR', 'SHELL', 'LANG', 'LC_ALL'] if key in parent}
    return environment | {
        'PATH': '/opt/homebrew/bin:' + str(home / 'Library/Android/sdk/platform-tools') + ':' + parent.get('PATH', os.defpath),
        'JAVA_HOME': '/opt/homebrew/opt/openjdk@17',
        'ANDROID_HOME': str(home / 'Library/Android/sdk'),
        'IOS_DEVICE': ios_device,
        'ANDROID_SERIAL': android_device,
        'CARGO_TARGET_DIR': str(ROOT / 'target'),
    }


def require_fork_approval(policy):
    if policy.get('approval_policy') != 'all_external_contributors':
        raise RuntimeError('Runner requires approval_policy=all_external_contributors; refusing to register')


def dispatch(gh, label, environment):
    subprocess.run([gh, 'workflow', 'run', 'integration.yml', '--repo', REPOSITORY,
                    '--ref', 'main', '--field', 'runner_label=' + label],
                   env=environment, check=True, timeout=60)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ios-device', required=True)
    parser.add_argument('--android-device', default='emulator-5554')
    options = parser.parse_args()
    environment = child_environment(options.ios_device, options.android_device, os.environ)
    control_environment = os.environ | {'PATH': environment['PATH']}
    gh = shutil.which('gh', path=control_environment['PATH'])
    if not gh:
        raise RuntimeError('GitHub CLI is required with repository admin authentication')
    policy = json.loads(subprocess.check_output([gh, 'api',
        f'repos/{REPOSITORY}/actions/permissions/fork-pr-contributor-approval'], env=control_environment, text=True))
    require_fork_approval(policy)
    label = 'bark-bolt-regtest-' + uuid.uuid4().hex
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
            f'repos/{REPOSITORY}/actions/runners/registration-token'], env=control_environment, text=True))['token']
        name = 'bark-bolt-ephemeral-' + directory.name.removeprefix('bark-bolt-ci-runner-')
        configured = subprocess.run([str(directory / 'config.sh'), '--unattended', '--ephemeral',
            '--url', 'https://github.com/' + REPOSITORY, '--token', token,
            '--name', name, '--no-default-labels', '--labels', label, '--work', '_work'],
            cwd=directory, env=environment, text=True, capture_output=True, timeout=120)
        if configured.returncode:
            raise RuntimeError((configured.stdout + configured.stderr).replace(token, '<redacted>'))
        print(f'Ephemeral runner {name} will run integration.yml on main in {REPOSITORY}.', flush=True)
        try:
            dispatch(gh, label, control_environment)
            subprocess.run([str(directory / 'run.sh'), '--once'], cwd=directory,
                           env=environment, check=True, timeout=7200)
        finally:
            # Ephemeral runners unregister themselves after the job. Remove a
            # registration left by an interrupted listener before deleting it.
            runners = json.loads(subprocess.check_output([gh, 'api', f'repos/{REPOSITORY}/actions/runners'],
                                env=control_environment, text=True))['runners']
            for runner in runners:
                if runner['name'] == name:
                    subprocess.run([gh, 'api', '--method', 'DELETE',
                                    f"repos/{REPOSITORY}/actions/runners/{runner['id']}"], env=control_environment, check=True)


if __name__ == '__main__':
    main()
