#!/usr/bin/env python3
"""Drive actual installed native apps through debug-only SDK test commands."""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import time
import regtest

BUNDLE = 'io.github.niteshbalusu11.bark.example'
ROOT = Path(__file__).resolve().parents[1]


def run(args, *, env=None, check=True):
    result = subprocess.run(args, text=True, capture_output=True, env=env, timeout=180)
    if check and result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result.stdout.strip()


class App:
    def __init__(self, platform, device, account):
        self.platform, self.device, self.account = platform, device, account
        self.adb = [shutil.which('adb') or str(Path.home() / 'Library/Android/sdk/platform-tools/adb'), '-s', device]

    def command(self, action, *, allow_error=False, **values):
        request = json.dumps(dict(action=action, server='http://127.0.0.1:' + os.environ.get('ARK_PORT', '3635'), bitcoin='http://127.0.0.1:' + os.environ.get('BITCOIN_PORT', '19443')) | values, separators=(',', ':'))
        if self.platform == 'ios':
            run(['xcrun', 'simctl', 'terminate', self.device, BUNDLE], check=False)
            container = Path(run(['xcrun', 'simctl', 'get_app_container', self.device, BUNDLE, 'data']))
            output = container / 'Documents/automation-result.json'
            output.unlink(missing_ok=True)
            env = os.environ | {'SIMCTL_CHILD_BARK_RUN_ID': self.account, 'SIMCTL_CHILD_BARK_COMMAND': request}
            run(['xcrun', 'simctl', 'launch', self.device, BUNDLE], env=env)
            read = lambda: json.loads(output.read_text()) if output.exists() else None
        else:
            run(self.adb + ['shell', 'am', 'force-stop', BUNDLE])
            run(self.adb + ['shell', 'run-as', BUNDLE, 'rm', '-f', 'files/automation-result.json'])
            run(self.adb + ['shell', 'am', 'start', '-W', '-n', BUNDLE + '/.MainActivity',
                            '--es', 'runId', self.account, '--es', 'command', shlex.quote(request)])
            def read():
                value = run(self.adb + ['shell', 'run-as', BUNDLE, 'cat', 'files/automation-result.json'], check=False)
                return json.loads(value) if value.startswith('{') else None
        end = time.monotonic() + 150
        while time.monotonic() < end:
            try:
                result = read()
                if result:
                    if not allow_error and not result.get('ok'):
                        raise RuntimeError(f'{self.platform} {action}: {result}')
                    print(json.dumps({'platform': self.platform, 'action': action, 'result': result}), flush=True)
                    return result
            except json.JSONDecodeError:
                pass
            time.sleep(0.5)
        raise RuntimeError(f'{self.platform} {action}: app produced no result in 150s')


def smoke(apps):
    state = {}
    for app in apps:
        state[app.platform] = app.command('open')
        app.command('cancel')
        failed = app.command('send', destination='invalid', amount=1000, allow_error=True)
        assert not failed['ok'] and failed.get('error'), 'Invalid address must fail with a message'
        address = state[app.platform]['onchainAddress']
        regtest.fund(address, 0.1)
    regtest.mine(6)
    for app in apps:
        app.command('sync')
        board = app.command('board', amount=2000000)
        assert board['notifications'] > 0, 'Board must emit real wallet notifications'
    regtest.mine(6)
    for app in apps:
        result = regtest.wait_for(f'{app.platform} board', lambda: app.command('sync'),
                                 lambda r: r['spendable'] >= 1900000)
        state[app.platform] = result
    for sender in apps:
        receiver = next((app for app in apps if app is not sender), None)
        target = state[receiver.platform]['arkAddress'] if receiver else regtest.bark('address')
        before = state[sender.platform]['spendable']
        sent = sender.command('send', destination=target, amount=10000)
        assert sent['spendable'] < before
        if receiver:
            previous = state[receiver.platform]['spendable']
            received = regtest.wait_for('Ark receive', lambda: receiver.command('sync'), lambda r: r['spendable'] >= previous + 10000)
            state[receiver.platform] = received
        state[sender.platform] = sent
    for app in apps:
        again = app.command('open')
        assert again['fingerprint'] == state[app.platform]['fingerprint']
        assert again['spendable'] == state[app.platform]['spendable']
        assert again['historyCount'] >= 2
    return state


def extended(apps):
    for app in apps:
        invoice = regtest.lnd('addinvoice', '--amt=1000')
        paid = app.command('send', destination=invoice['payment_request'], amount=1000)
        invoice_hash = invoice['r_hash']
        assert regtest.lnd('lookupinvoice', invoice_hash)['settled']
        inbound = app.command('invoice', amount=2000)
        regtest.lnd('payinvoice', '--force', inbound['invoice'])
        claimed = app.command('claim')
        # The daemon may already own the claim; wait for its committed balance.
        regtest.wait_for('Lightning receive credit', lambda: app.command('sync'),
                         lambda result: result['spendable'] > paid['spendable'])
        payments = regtest.lnd('listpayments')['payments']
        assert any(p['payment_hash'] == inbound['paymentHash'] and p['status'] == 'SUCCEEDED' for p in payments)
        app.command('refresh')
        regtest.mine(6)
        app.command('sync')
        address = regtest.bitcoin('getnewaddress', wallet='miner')
        original = app.command('sync')
        restored = app.command('restore')
        assert restored['spendable'] == original['spendable'] > 0, 'Seed recovery must recover spendable Ark funds'
        offboard = app.command('offboard', destination=address)
        regtest.mine(6)
        assert regtest.bitcoin('getrawtransaction', offboard['txid'], 'true')['confirmations'] > 0
        app.command('sync')
        # Ensure an independent remaining VTXO exists for unilateral exit.
        app.command('board', amount=500000)
        regtest.mine(6)
        app.command('sync')
        estimate = app.command('estimate')
        assert estimate['totalFee'] > 0
        assert estimate['totalFee'] == estimate['broadcastFee'] + estimate['claimFee']
        app.command('startExit')
        for _ in range(20):
            progress = app.command('progressExit')
            if progress['claimable']:
                break
            regtest.mine(6)
        else:
            raise AssertionError('Exit failed to mature within 120 blocks')
        claim = app.command('drainExit', destination=address)
        regtest.mine(6)
        assert regtest.bitcoin('getrawtransaction', claim['txid'], 'true')['confirmations'] > 0
        old = app.command('sync')
        restored = app.command('restore')
        assert restored['fingerprint'] == old['fingerprint']
        assert restored['onchain'] > 0, 'Restored onchain scan must find prior funds'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=['ios', 'android', 'both'], default='both')
    parser.add_argument('--ios-device', default=os.environ.get('IOS_DEVICE'))
    parser.add_argument('--android-device', default=os.environ.get('ANDROID_SERIAL', 'emulator-5554'))
    parser.add_argument('--run-id', default='integration-' + str(int(time.time())))
    parser.add_argument('--action', help='Execute one debug command instead of the full acceptance sequence')
    parser.add_argument('--extended', action='store_true')
    parser.add_argument('--request', default='{}', help='JSON values for a single action')
    options = parser.parse_args()
    apps = []
    if options.platform in ['ios', 'both']:
        if not options.ios_device:
            parser.error('--ios-device or IOS_DEVICE is required for iOS')
        apps.append(App('ios', options.ios_device, options.run_id))
    if options.platform in ['android', 'both']:
        app = App('android', options.android_device, options.run_id)
        for port in [os.environ.get('ARK_PORT', '3635'), os.environ.get('BITCOIN_PORT', '19443')]:
            run(app.adb + ['reverse', 'tcp:' + port, 'tcp:' + port])
        apps.append(app)
    if options.action:
        for app in apps:
            app.command(options.action, **json.loads(options.request))
        return
    regtest.bootstrap()
    smoke(apps)
    if options.extended:
        extended(apps)
    for app in apps:
        unavailable = App(app.platform, app.device, options.run_id + '-unavailable')
        failed = unavailable.command('open', server='http://127.0.0.1:1', allow_error=True)
        assert not failed['ok'] and failed.get('error')
        unavailable.command('open')
    print(json.dumps({'passed': True, 'platforms': [a.platform for a in apps], 'extended': options.extended, 'run_id': options.run_id}), flush=True)


if __name__ == '__main__':
    main()
