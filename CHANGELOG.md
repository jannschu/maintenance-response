# Changelog

## [0.1.2](https://github.com/jannschu/maintenance-response/compare/v0.1.1...v0.1.2) (2025-08-17)

### Bug Fixes

* Use static reference to WASM file

### Build

* Add wasm-opt call to release build workflow
* Bump http-wasm-guest dependency

## [0.1.1](https://github.com/jannschu/maintenance-response/compare/v0.1.0...v0.1.1) (2025-07-14)

### Bug Fixes

* Fix example in README to include version

### Dependencies

* Update dependency

## [0.1.0](https://github.com/jannschu/maintenance-response/releases/tag/v0.1.0) (2025-07-11)

### Features

* Initial implementation as a Traefik WASM plugin
* Custom response content based on `Accept` header (content negotiation)
* Request filtering using Cloudflare Wirefilter syntax
* Rewrite in Rust

### Tests

* Add end-to-end tests for enabling and content negotiation
* Add tests for filter language

### Build

* Add CI workflow for running tests
* Add separate build and release workflow
