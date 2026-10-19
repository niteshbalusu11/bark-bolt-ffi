#!/usr/bin/env python3
"""Isolated, repeatable Bark 0.7.1 RegTest stack. Never deletes wallet volumes."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
COMPOSE = ['docker', 'compose', '-f', str(ROOT / 'regtest/compose.yaml')]


def command(args, *, parsed=True):
    result = subprocess.run(COMPOSE + args, text=True, capture_output=True, timeout=180)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    value = result.stdout.strip()
    if parsed and value:
        try:
            return json.loads(value)
        except json.JSONDecodeError:
            return value
    return value


def bitcoin(*args, wallet=None):
    return command(['exec', '-T', 'bitcoind', 'bitcoin-cli', '-regtest',
                    '-rpcuser=second', '-rpcpassword=ark'] +
                   ([f'-rpcwallet={wallet}'] if wallet else []) + list(map(str, args)))


def server(*args):
    return command(['exec', '-T', 'captaind', 'captaind', 'rpc'] + list(args))


def cln(*args):
    return command(['exec', '-T', 'cln', 'lightning-cli', '--regtest'] + list(map(str, args)))


def lnd(*args):
    return command(['exec', '-T', 'lnd', 'lncli', '--network=regtest'] + list(map(str, args)))


def bark(*args, parsed=True):
    return command(['run', '--rm', '-T', 'bark', '--datadir', '/wallets/reference',
                    '--no-logfile'] + list(map(str, args)), parsed=parsed)


def wait_for(label, action, ready=lambda value: bool(value), timeout=120):
    end = time.monotonic() + timeout
    last = None
    while time.monotonic() < end:
        try:
            last = action()
            if ready(last):
                return last
        except (RuntimeError, json.JSONDecodeError) as error:
            last = str(error)
        time.sleep(1)
    raise RuntimeError(f'{label} not ready after {timeout}s: {last}')


def mine(blocks):
    return bitcoin('generatetoaddress', blocks, bitcoin('getnewaddress', wallet='miner'))


def fund(address, amount):
    return bitcoin('sendtoaddress', address, amount, wallet='miner')


def bootstrap():
    command(['up', '-d'], parsed=False)
    wait_for('Bitcoin', lambda: bitcoin('getblockchaininfo'))
    wallets = bitcoin('listwallets')
    if 'miner' not in wallets:
        existing = bitcoin('listwalletdir')['wallets']
        bitcoin('loadwallet' if any(w['name'] == 'miner' for w in existing) else 'createwallet', 'miner')
    if bitcoin('getbalance', wallet='miner') < 50:
        mine(110)
    wallet = wait_for('Ark server', lambda: server('wallet'))
    if wallet['rounds']['trusted_balance'] < 100000000:
        fund(wallet['rounds']['address'], 25)
        mine(6)
    wait_for('LND', lambda: lnd('getinfo'), lambda v: v.get('synced_to_chain', False))
    peer_info = wait_for('CLN', lambda: cln('getinfo'))
    peer = peer_info['id']
    peer_port = peer_info['binding'][0]['port']
    channels = lnd('listchannels')['channels']
    if not any(c['remote_pubkey'] == peer for c in channels):
        pending = lnd('pendingchannels')['pending_open_channels']
        if not pending:
            fund(lnd('newaddress', 'p2wkh')['address'], 1)
            fund(cln('newaddr')['bech32'], 1)
            mine(6)
            wait_for('LND chain sync', lambda: lnd('walletbalance'),
                     lambda b: int(b['confirmed_balance']) >= 2000000)
            wait_for('LND chain sync after mining', lambda: lnd('getinfo'),
                     lambda v: v.get('synced_to_chain', False) and v['block_height'] == bitcoin('getblockcount'))
            if not any(p['pub_key'] == peer for p in lnd('listpeers')['peers']):
                lnd('connect', f'{peer}@cln:{peer_port}')
            lnd('openchannel', f'--node_key={peer}', '--local_amt=10000000', '--push_amt=5000000')
        mine(6)
    wait_for('Lightning channel', lambda: lnd('listchannels'),
             lambda c: any(x['active'] and x['remote_pubkey'] == peer for x in c['channels']))
    # Opening an existing wallet never replaces its seed or database.
    exists = command(['run', '--rm', '-T', '--entrypoint', 'sh', 'bark', '-c',
                      'test -f /wallets/reference/mnemonic && echo yes || true'], parsed=False)
    if not exists:
        bark('create', '--regtest', '--ark', 'http://captaind:3535',
             '--bitcoind', 'http://bitcoind:18443', '--bitcoind-user', 'second',
             '--bitcoind-pass', 'ark', parsed=False)
    print(json.dumps({'bitcoin_height': bitcoin('getblockcount'),
                      'lightning_channels': len(lnd('listchannels')['channels']),
                      'ark_wallet': wallet}, indent=2))


def reference_smoke():
    bootstrap()
    if bark('balance')['spendable_sat'] < 100000:
        address = bark('onchain', 'address')['address']
        fund(address, 0.05)
        mine(6)
        print(bark('board', '1000000sat'))
        mine(6)
    balance = wait_for('reference board', lambda: bark('balance'),
                       lambda b: b['spendable_sat'] >= 100000)
    print(json.dumps({'reference_balance': balance, 'channels': lnd('listchannels')}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['start', 'stop', 'status', 'logs', 'mine', 'fund',
                                        'bitcoin', 'bark', 'server', 'cln', 'lnd', 'reference-smoke'])
    parser.add_argument('args', nargs=argparse.REMAINDER)
    options = parser.parse_args()
    if options.action == 'reference-smoke':
        reference_smoke()
        return
    if options.action == 'start':
        bootstrap()
        return
    if options.action in ('stop', 'status', 'logs'):
        args = {'stop': ['stop'], 'status': ['ps'], 'logs': ['logs', '--tail=100']}[options.action]
        print(command(args + options.args, parsed=False))
        return
    if options.action == 'mine':
        result = mine(int(options.args[0]) if options.args else 1)
    elif options.action == 'fund':
        result = fund(*options.args)
    else:
        result = globals()[options.action](*options.args)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        sys.exit(str(error))
