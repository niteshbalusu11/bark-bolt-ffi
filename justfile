# RegTest commands adapted from Nitro Ark's Justfile (MIT; see regtest/LICENSE).
set positional-arguments := true

default:
    @just --list

setup-everything:
    @./scripts/ark-dev.sh setup-everything

up:
    @./scripts/ark-dev.sh up

down *args:
    @./scripts/ark-dev.sh down "$@"

stop:
    @./scripts/ark-dev.sh stop

status:
    @./scripts/ark-dev.sh status

logs *args:
    @./scripts/ark-dev.sh logs "$@"

bark *args:
    @./scripts/ark-dev.sh bark "$@"

aspd *args:
    @./scripts/ark-dev.sh aspd "$@"

bcli *args:
    @./scripts/ark-dev.sh bcli "$@"

lncli *args:
    @./scripts/ark-dev.sh lncli "$@"

cln *args:
    @./scripts/ark-dev.sh cln "$@"

create-wallet:
    @./scripts/ark-dev.sh create-wallet

create-bark-wallet:
    @./scripts/ark-dev.sh create-bark-wallet

generate blocks="101":
    @./scripts/ark-dev.sh generate "$1"

fund-aspd amount:
    @./scripts/ark-dev.sh fund-aspd "$1"

send-to address amount:
    @./scripts/ark-dev.sh send-to "$1" "$2"

setup-lightning:
    @./scripts/ark-dev.sh setup-lightning-channels

# Mobile JSON polling/assertions and bounded Actions runner registration.
wait-devices *args:
    @python3 scripts/wait-devices.py "$@"

test-regtest *args:
    @python3 scripts/app-regtest.py "$@"

ci-regtest *args:
    @python3 scripts/ci-regtest-runner.py "$@"

build-ios *args:
    @./scripts/build-sdk.sh apple "$@"

build-android *args:
    @./scripts/build-sdk.sh android "$@"
