# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

SwiftUI on iOS 16+ and Jetpack Compose on Android API24+, consuming this repo's
BoltFFI SDK. Confirmed by the migration brief.

## Users

Developers validating Bark's Swift and Kotlin SDKs on local RegTest.

## Product Purpose

Demonstrate real wallet creation, recovery, boarding, Ark and Lightning payments,
history, maintenance and exits from native apps.

## Operating Context

An isolated Bitcoin/Ark/CLN/LND RegTest stack runs on the developer machine.
Apps persist private wallet data and report real network outcomes.

## Capabilities and Constraints

Breaking binding API changes are accepted. Only Swift and Kotlin are consumer
products. Seeds must stay out of logs. Apps and SDK builds run in GitHub Actions.

## Open Decisions

The user specified no visual brand; native system controls and a restrained
single tint are implementation assumptions. These examples target phones first.
