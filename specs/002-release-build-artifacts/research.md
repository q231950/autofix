# Research: Release Build Artifacts

**Feature**: 002-release-build-artifacts
**Date**: 2025-12-14

## Overview

This document captures research decisions for implementing automated GitHub release creation with precompiled Apple Silicon binaries. The workflow must trigger on version tags, build the Rust binary for aarch64-apple-darwin, package it, generate checksums, and create a GitHub release.

## Key Decisions

### 1. GitHub Actions Workflow Trigger

**Decision**: Use `on.push.tags` with pattern matching for `v*.*.*`

**Rationale**:
- GitHub Actions provides native support for tag-based triggers
- Pattern matching (`v*.*.*`) ensures only semantic version tags trigger releases
- Tag triggers are distinct from branch pushes, preventing accidental releases
- This matches industry standard practices (e.g., rust-lang/rust, tokio-rs/tokio)

**Alternatives Considered**:
- **Manual workflow dispatch**: Rejected because it requires manual intervention, defeating automation purpose
- **Release creation triggers**: Rejected because we want to create the release, not react to it
- **Branch-based triggers with tag detection**: Rejected as unnecessarily complex

**Implementation Notes**:
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
```

### 2. Cross-Compilation for Apple Silicon

**Decision**: Use GitHub Actions macOS runners with native aarch64-apple-darwin compilation

**Rationale**:
- GitHub provides macOS runners with Apple Silicon support (macos-14 and later use M1 chips)
- Native compilation is simpler and more reliable than cross-compilation from Linux
- Rust toolchain fully supports aarch64-apple-darwin target
- No need for cross-compilation toolchains or additional dependencies

**Alternatives Considered**:
- **Cross-compile from Linux**: Rejected due to complexity of setting up macOS SDK and linker on Linux
- **Use docker containers**: Rejected because macOS cannot run in Docker on GitHub Actions
- **Build on Intel and cross-compile**: Rejected because native builds are simpler and GitHub provides M1 runners

**Implementation Notes**:
- Use `runs-on: macos-14` or later for Apple Silicon runners
- Add target with `rustup target add aarch64-apple-darwin` (may already be default on M1 runners)
- Build with `cargo build --release --target aarch64-apple-darwin`

### 3. Binary Packaging and Compression

**Decision**: Use zip format with binary and optional README/LICENSE

**Rationale**:
- Zip is universally supported on macOS (built-in Archive Utility)
- Simple to create using standard `zip` command available on GitHub runners
- Matches user expectations for downloadable macOS software
- Smaller than tar.gz for single binary due to zip compression

**Alternatives Considered**:
- **tar.gz**: Rejected because zip is more native to macOS users
- **DMG image**: Rejected as overkill for a CLI binary (no graphical installer needed)
- **Uncompressed binary**: Rejected because compression reduces download size and bandwidth

**Implementation Notes**:
```bash
cd target/aarch64-apple-darwin/release
zip autofix-macos-aarch64.zip autofix
```

### 4. Checksum Generation

**Decision**: Generate SHA256 checksums using `shasum -a 256`

**Rationale**:
- SHA256 is cryptographically secure and widely recognized
- `shasum` is pre-installed on macOS GitHub runners
- Easy for users to verify downloads using built-in macOS tools
- Matches Rust ecosystem conventions (Cargo uses SHA256 for package verification)

**Alternatives Considered**:
- **MD5**: Rejected due to cryptographic weaknesses
- **SHA512**: Rejected as unnecessary; SHA256 provides sufficient security
- **GPG signatures**: Deferred to future enhancement (requires key management infrastructure)

**Implementation Notes**:
```bash
shasum -a 256 autofix-macos-aarch64.zip > autofix-macos-aarch64.zip.sha256
```

### 5. Release Creation Method

**Decision**: Use `softprops/action-gh-release` GitHub Action

**Rationale**:
- Well-maintained action (5k+ stars, used by major projects)
- Automatically handles release creation and file uploads
- Supports release notes generation from tags and commits
- Handles idempotency (won't duplicate releases)
- Simpler than using GitHub CLI or REST API directly

**Alternatives Considered**:
- **GitHub CLI (`gh release create`)**: Rejected because action provides better error handling and GitHub integration
- **actions/create-release + actions/upload-release-asset**: Rejected because deprecated by GitHub
- **Direct GitHub API calls**: Rejected as unnecessarily complex

**Implementation Notes**:
```yaml
- uses: softprops/action-gh-release@v1
  with:
    files: |
      target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip
      target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip.sha256
    body_path: RELEASE_NOTES.md
    draft: false
    prerelease: false
```

### 6. Build Metadata in Release Notes

**Decision**: Generate release notes file with build metadata template

**Rationale**:
- Provides transparency about build environment and reproducibility
- Helps users and maintainers debug issues related to specific builds
- Aligns with FR-006 requirement for build metadata
- Can be automated using environment variables in GitHub Actions

**Alternatives Considered**:
- **Hardcode in workflow**: Rejected because metadata should be dynamic
- **Use git commit messages**: Rejected because it doesn't include build environment details
- **No release notes**: Rejected because violates FR-006 requirement

**Implementation Notes**:
Create RELEASE_NOTES.md dynamically:
```bash
cat > RELEASE_NOTES.md <<EOF
## Build Information

- **Build Date**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
- **Commit SHA**: $GITHUB_SHA
- **Rust Version**: $(rustc --version)
- **Target**: aarch64-apple-darwin

## Checksum

\`\`\`
$(cat autofix-macos-aarch64.zip.sha256)
\`\`\`

## Installation

1. Download autofix-macos-aarch64.zip
2. Extract the archive
3. Move the binary to your PATH (e.g., /usr/local/bin)
4. Verify with: \`autofix --version\`
EOF
```

### 7. Workflow Failure Handling

**Decision**: Rely on GitHub Actions default failure behavior with explicit build verification

**Rationale**:
- GitHub Actions automatically marks workflows as failed if any step fails
- Non-zero exit codes from `cargo build` will prevent release creation
- No partial releases can occur because file uploads happen after build succeeds
- Maintainers receive email notifications on workflow failures

**Alternatives Considered**:
- **Custom failure notifications**: Deferred to future enhancement (Slack/Discord integration)
- **Retry logic**: Rejected because build failures usually require code changes, not retries
- **Rollback mechanism**: Not applicable (failed workflows don't create releases)

**Implementation Notes**:
- Use `cargo build --release` without `|| true` to ensure failures propagate
- Add explicit build verification step if needed: `test -f target/aarch64-apple-darwin/release/autofix`

### 8. Rust Build Configuration

**Decision**: Use `cargo build --release` with default optimization

**Rationale**:
- Release mode provides optimized binaries with debug info stripped
- Default optimization level (opt-level=3) balances size and performance
- No special build flags needed for basic CLI tool distribution

**Alternatives Considered**:
- **Custom optimization**: Deferred to future enhancement (could use opt-level='z' for smaller binaries)
- **LTO (Link-Time Optimization)**: Deferred because build time impact may exceed GitHub Actions limits
- **Strip symbols separately**: Rejected because release mode already strips debug symbols

**Implementation Notes**:
```bash
cargo build --release --target aarch64-apple-darwin
```

## Technology Stack Summary

| Component | Choice | Version/Details |
|-----------|--------|-----------------|
| CI Platform | GitHub Actions | Latest (ubuntu/macos runners) |
| Workflow Trigger | Tag push | Pattern: `v*.*.*` |
| Build Runner | macOS 14+ | Apple Silicon (M1) native |
| Rust Toolchain | Stable | Latest stable via rustup |
| Build Target | aarch64-apple-darwin | Native Apple Silicon |
| Archive Format | ZIP | Standard macOS format |
| Checksum Algorithm | SHA256 | Via `shasum -a 256` |
| Release Action | softprops/action-gh-release | v1 (latest) |

## Open Questions (None)

All technical decisions have been resolved. No clarifications needed for implementation.

## References

- [GitHub Actions Documentation - Workflow Triggers](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#push)
- [Rust Platform Support - aarch64-apple-darwin](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)
- [GitHub Actions - macOS Runners](https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-runners-and-hardware-resources)
