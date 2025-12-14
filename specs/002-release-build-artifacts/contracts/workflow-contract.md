# GitHub Actions Workflow Contract

**Feature**: 002-release-build-artifacts
**Date**: 2025-12-14

## Overview

This document specifies the contract for the GitHub Actions workflow that automates release creation. Since this is infrastructure code rather than an API, we define the workflow's inputs, outputs, and behavior contract.

## Workflow Specification

### Name
`release.yml` (GitHub Actions workflow file)

### Trigger Contract

**Event**: `push` on `tags` matching pattern `v*.*.*`

**Input** (Implicit from GitHub):
- `GITHUB_REF`: The git ref that triggered the workflow (e.g., `refs/tags/v1.2.3`)
- `GITHUB_SHA`: The commit SHA associated with the tag
- `GITHUB_REPOSITORY`: Repository name (e.g., `owner/repo`)

**Preconditions**:
- Tag MUST match semantic version pattern `v[0-9]+.[0-9]+.[0-9]+`
- Tag MUST point to a valid commit in the repository
- Repository MUST contain a valid Rust project with `Cargo.toml`

**Example Triggers**:
```bash
# Valid triggers
git tag v1.0.0 && git push origin v1.0.0
git tag v2.3.15 && git push origin v2.3.15

# Invalid (won't trigger)
git tag release-1.0 && git push origin release-1.0  # Wrong pattern
git push origin main                                 # Not a tag
```

## Workflow Steps Contract

### Step 1: Checkout Code

**Action**: `actions/checkout@v4`

**Inputs**:
- `fetch-depth`: 0 (full history for build metadata)

**Outputs**:
- Repository source code available in workspace

**Success Criteria**:
- Exit code 0
- `.git` directory exists
- `Cargo.toml` exists in workspace root

### Step 2: Setup Rust Toolchain

**Action**: `actions-rs/toolchain@v1` or `dtolnay/rust-toolchain@stable`

**Inputs**:
- `toolchain`: stable
- `target`: aarch64-apple-darwin
- `override`: true

**Outputs**:
- Rust toolchain installed
- `cargo` command available
- `aarch64-apple-darwin` target installed

**Success Criteria**:
- Exit code 0
- `cargo --version` succeeds
- `rustup target list --installed` includes `aarch64-apple-darwin`

### Step 3: Build Binary

**Command**: `cargo build --release --target aarch64-apple-darwin`

**Inputs**:
- Source code from workspace
- Rust toolchain from Step 2

**Outputs**:
- Binary file: `target/aarch64-apple-darwin/release/autofix`

**Success Criteria**:
- Exit code 0 (compilation succeeded)
- Binary file exists and has execute permissions
- Binary is valid Mach-O for aarch64-apple-darwin

**Error Handling**:
- Non-zero exit → workflow fails, no release created
- Missing dependencies → failure message in workflow log
- Compile errors → error details in workflow log

### Step 4: Package Binary

**Command**:
```bash
cd target/aarch64-apple-darwin/release
zip autofix-macos-aarch64.zip autofix
```

**Inputs**:
- Binary from Step 3: `autofix`

**Outputs**:
- Archive file: `target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip`

**Success Criteria**:
- Exit code 0
- ZIP file exists and is valid ZIP archive
- Archive contains `autofix` file with executable permissions

### Step 5: Generate Checksum

**Command**: `shasum -a 256 autofix-macos-aarch64.zip > autofix-macos-aarch64.zip.sha256`

**Inputs**:
- Archive from Step 4: `autofix-macos-aarch64.zip`

**Outputs**:
- Checksum file: `target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip.sha256`
- File format: `<64-char-hex-hash>  autofix-macos-aarch64.zip`

**Success Criteria**:
- Exit code 0
- Checksum file exists
- File contains valid SHA256 hex string (64 characters)

### Step 6: Generate Release Notes

**Command**: Bash script to create `RELEASE_NOTES.md`

**Inputs**:
- `GITHUB_SHA`: Commit hash
- Current date/time
- Rust version from `rustc --version`
- Checksum from Step 5

**Outputs**:
- File: `RELEASE_NOTES.md`

**Template**:
```markdown
## Build Information

- **Build Date**: <ISO 8601 UTC timestamp>
- **Commit SHA**: <GITHUB_SHA>
- **Rust Version**: <rustc --version output>
- **Target**: aarch64-apple-darwin

## Checksum

```
<contents of .sha256 file>
```

## Installation

1. Download autofix-macos-aarch64.zip
2. Extract: `unzip autofix-macos-aarch64.zip`
3. Move to PATH: `mv autofix /usr/local/bin/`
4. Verify: `autofix --version`
```

**Success Criteria**:
- File created successfully
- All placeholders filled with actual values
- Valid markdown syntax

### Step 7: Create GitHub Release

**Action**: `softprops/action-gh-release@v1`

**Inputs**:
```yaml
files: |
  target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip
  target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip.sha256
body_path: RELEASE_NOTES.md
draft: false
prerelease: false
token: ${{ secrets.GITHUB_TOKEN }}
```

**Outputs**:
- GitHub Release created at `https://github.com/<owner>/<repo>/releases/tag/<tag>`
- Assets uploaded: zip file + checksum file
- Release notes populated from RELEASE_NOTES.md

**Success Criteria**:
- Exit code 0
- Release visible on GitHub releases page
- Both files downloadable from release
- Release notes rendered correctly

**Error Handling**:
- Duplicate release → action handles idempotency (updates existing)
- Upload failure → retry logic in action
- Permission error → workflow fails with clear message

## Workflow Outputs

### On Success

**GitHub Release Created**:
- URL: `https://github.com/<owner>/<repo>/releases/tag/<tag>`
- Status: Published (not draft)
- Assets: 2 files
  1. `autofix-macos-aarch64.zip` (executable binary)
  2. `autofix-macos-aarch64.zip.sha256` (checksum)
- Release notes: Build metadata + checksum + installation instructions

**Notifications**:
- Workflow success notification to maintainers
- Release notification to repository watchers (per GitHub settings)

### On Failure

**No Release Created**:
- Workflow status: Failed
- GitHub Release: Not created (no partial releases)
- Workflow logs: Available for debugging

**Notifications**:
- Workflow failure email to workflow author
- Failure status visible on GitHub Actions tab

## Performance Contract

**Timing Guarantees** (per SC-001):
- Total workflow duration: < 10 minutes
- Typical duration: 3-5 minutes (varies by codebase size)

**Breakdown** (estimated):
1. Checkout: 10-30 seconds
2. Rust setup: 30-60 seconds
3. Build: 2-4 minutes (depends on crate size and dependencies)
4. Package + checksum: 5-10 seconds
5. Release creation: 30-60 seconds (upload time depends on binary size)

**Timeout**: Set workflow timeout to 15 minutes (50% buffer)

## Security Contract

**Permissions Required**:
- `contents: write` - To create releases and upload assets
- `GITHUB_TOKEN` - Automatically provided by GitHub Actions

**Security Guarantees**:
- No API keys hardcoded in workflow file
- No secrets logged to workflow output
- Artifacts are public (release is public)
- Checksums allow verification of download integrity

**No Security Features** (deferred to future):
- GPG signing of binaries
- SLSA provenance attestation
- Artifact encryption

## Testing Contract

**Manual Testing Required**:
1. Create test tag: `git tag v0.0.1-test && git push origin v0.0.1-test`
2. Verify workflow triggers and completes
3. Download artifacts from release
4. Verify checksum: `shasum -a 256 -c autofix-macos-aarch64.zip.sha256`
5. Extract and run binary: `./autofix --version`
6. Clean up test release

**Automated Testing**:
- No automated tests for workflow itself (GitHub Actions limitation)
- Workflow acts as integration test for build process

## Error Scenarios

| Scenario | Detection | Behavior | Recovery |
|----------|-----------|----------|----------|
| Build fails | `cargo build` exit code ≠ 0 | Workflow fails at Step 3 | Fix code, push new tag |
| Binary not found | File check after build | Workflow fails | Investigate build output |
| Zip creation fails | `zip` exit code ≠ 0 | Workflow fails at Step 4 | Check binary permissions |
| Checksum fails | `shasum` exit code ≠ 0 | Workflow fails at Step 5 | Check zip file exists |
| Release creation fails | Action error | Workflow fails at Step 7 | Check permissions, retry tag |
| Duplicate tag | Tag already exists | Workflow updates existing release | Delete tag if needed |
| Network failure | Action timeout/error | Workflow fails with retry | Re-run workflow |

## Compliance with Functional Requirements

- **FR-001**: ✅ Workflow triggers on `v*.*.*` tags only
- **FR-002**: ✅ Builds for aarch64-apple-darwin target
- **FR-003**: ✅ Creates ZIP archive
- **FR-004**: ✅ Creates GitHub release with tag
- **FR-005**: ✅ Attaches ZIP to release
- **FR-006**: ✅ Includes build metadata in release notes
- **FR-007**: ✅ Generates and includes SHA256 checksum
- **FR-008**: ✅ Only triggers on tags (not branches)
- **FR-009**: ✅ Fails gracefully (no partial releases)
- **FR-010**: ✅ Binary targets Apple Silicon macOS

## Version

**Contract Version**: 1.0.0
**Last Updated**: 2025-12-14
