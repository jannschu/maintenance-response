# Changelog

## [0.1.4](https://github.com/jannschu/maintenance-response/compare/v0.1.3...v0.1.4) (2026-02-09)


### Bug Fixes

* version update in README ([36a4592](https://github.com/jannschu/maintenance-response/commit/36a459264199b465e5303204805aa400ba41844b))

## [0.1.3](https://github.com/jannschu/maintenance-response/compare/v0.1.2...v0.1.3) (2026-02-09)


### Features

* automate releases with release-please ([06842cf](https://github.com/jannschu/maintenance-response/commit/06842cfc7417bea2c960cf3964d7cf9a1632d5f6))


### Bug Fixes

* correct release-please config for README version and release title ([65010cc](https://github.com/jannschu/maintenance-response/commit/65010ccb6df4b3146b96013e7a4c9916124a55fd))
* release-please versioning ([07a0103](https://github.com/jannschu/maintenance-response/commit/07a01031b70ee9f04dd85358c45d94c19fb54b68))

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
