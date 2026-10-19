# Native device setup

Use Xcode's Simulator app and Android Studio's Device Manager to select and boot
an iPhone and an Android API35 phone. Local Apple silicon uses an arm64 Android
system image; GitHub-hosted Linux uses x86_64 with KVM.

For command-line Android setup, with ANDROID_HOME and JDK17 configured:

```sh
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" 'emulator' \
  'system-images;android-35;google_apis;arm64-v8a' 'platforms;android-35' \
  'build-tools;35.0.0' 'ndk;28.2.13676358'
"$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager" create avd \
  --name BarkRegtest --package 'system-images;android-35;google_apis;arm64-v8a' --device pixel_7
"$ANDROID_HOME/emulator/emulator" -avd BarkRegtest -gpu auto
```

Create the AVD once; reuse it subsequently. Find identifiers with
`xcrun simctl list devices available` and `adb devices`. Readiness is bounded:

```sh
python3 scripts/wait-devices.py --ios-device IOS_UDID --android-device emulator-5554
```

Build/install apps from README, then run scripts/app-regtest.py with those IDs.
The driver configures adb reverse for the selected RegTest ports. T3 users can
open both devices in the Device panel; verification in this migration used the
agent-device launcher returned by device_open for interactive navigation.

Full Actions integration needs these tools on a macOS ARM64 runner. For one
trusted repository-scoped job, without installing a service:

```sh
python3 scripts/ci-regtest-runner.py --ios-device IOS_UDID --android-device emulator-5554
# In another terminal, dispatch the workflow:
gh workflow run integration.yml
```

The helper requires authenticated GitHub CLI repository-admin access, registers
an ephemeral runner, accepts one job, unregisters and removes its temporary
files. It uses an isolated Compose project/ports; after uploading evidence,
the workflow removes its own per-run containers/volumes. Local `regtest.py stop`
preserves developer wallets. A permanent integration runner
can instead use the same bark-bolt-regtest label and documented prerequisites.
