# Data Model: Release Build Artifacts

**Feature**: 002-release-build-artifacts
**Date**: 2025-12-14

## Overview

This feature is purely infrastructure-focused (CI/CD automation). There are no traditional database entities or API data models. However, we document the conceptual data structures and artifacts involved in the release process.

## Conceptual Entities

### Release Tag

A git tag that triggers the release workflow.

**Attributes**:
- **Name**: String matching pattern `v*.*.*` (e.g., "v1.2.3")
- **Commit SHA**: Git commit hash the tag points to
- **Creation Timestamp**: When the tag was created
- **Author**: Git user who created the tag

**Validation Rules**:
- MUST match semantic version pattern `v[MAJOR].[MINOR].[PATCH]`
- MUST point to a valid commit in the repository
- MUST NOT already have an associated GitHub release (workflow handles idempotency)

**State**: Immutable once created

### Binary Artifact

The compiled executable produced by the workflow.

**Attributes**:
- **Filename**: `autofix` (pre-compression)
- **Target Architecture**: `aarch64-apple-darwin`
- **File Size**: Variable (depends on codebase size)
- **Permissions**: Executable (0755)
- **Build Configuration**: Release mode (optimized, stripped)

**Validation Rules**:
- MUST be a valid Mach-O executable for Apple Silicon
- MUST have execute permissions set
- MUST be runnable on macOS 11+ with Apple Silicon

**Relationships**:
- Produced by: Release Tag (via workflow)
- Packaged into: Archive Artifact

### Archive Artifact

The zipped binary package for distribution.

**Attributes**:
- **Filename**: `autofix-macos-aarch64.zip`
- **Format**: ZIP compression
- **Contents**: Single executable binary (`autofix`)
- **Compressed Size**: Variable (typically 1-5 MB for CLI tools)

**Validation Rules**:
- MUST be a valid ZIP archive
- MUST contain at least the `autofix` binary
- MUST be extractable with standard macOS Archive Utility

**Relationships**:
- Contains: Binary Artifact
- Associated with: Checksum Artifact, GitHub Release

### Checksum Artifact

SHA256 hash of the archive for verification.

**Attributes**:
- **Filename**: `autofix-macos-aarch64.zip.sha256`
- **Format**: Plain text, hex-encoded SHA256 hash
- **Content**: `<hash>  autofix-macos-aarch64.zip`
- **File Size**: ~80 bytes

**Validation Rules**:
- MUST be valid SHA256 hex encoding (64 hex characters)
- MUST match the actual SHA256 of the archive artifact
- MUST follow standard `shasum` output format

**Relationships**:
- Verifies: Archive Artifact

### Build Metadata

Information about the build environment and configuration.

**Attributes**:
- **Build Date**: ISO 8601 timestamp in UTC
- **Commit SHA**: Full git commit hash (40 characters)
- **Rust Version**: Output of `rustc --version`
- **Target Triple**: `aarch64-apple-darwin`
- **Workflow Run ID**: GitHub Actions run identifier
- **Runner OS**: macOS version of the GitHub Actions runner

**Validation Rules**:
- Build date MUST be within reasonable time of tag creation (< 1 hour)
- Commit SHA MUST match the tag's commit
- Rust version MUST be from stable channel

**Relationships**:
- Associated with: GitHub Release (embedded in release notes)

### GitHub Release

The release object created on GitHub's platform.

**Attributes**:
- **Tag Name**: String (matches Release Tag name)
- **Release Name**: String (typically same as tag name)
- **Release Notes**: Markdown text containing Build Metadata and checksums
- **Draft**: Boolean (false - published immediately)
- **Prerelease**: Boolean (false for stable releases)
- **Creation Timestamp**: When release was created by workflow
- **Assets**: Array of uploaded files

**Validation Rules**:
- Tag name MUST be unique across all releases in repository
- MUST have at least 2 assets (archive + checksum)
- Release notes MUST contain SHA256 checksum
- MUST NOT be a draft (published state)

**Relationships**:
- Triggered by: Release Tag
- Contains assets: Archive Artifact, Checksum Artifact
- Includes metadata: Build Metadata

## Data Flow

```text
1. Developer creates Release Tag (v1.2.3)
   ↓
2. GitHub Actions workflow triggered
   ↓
3. Workflow builds Binary Artifact (autofix)
   ↓
4. Binary packaged into Archive Artifact (autofix-macos-aarch64.zip)
   ↓
5. Checksum Artifact generated (autofix-macos-aarch64.zip.sha256)
   ↓
6. Build Metadata collected from environment
   ↓
7. GitHub Release created with:
   - Tag Name
   - Release Notes (containing Build Metadata + Checksum)
   - Attached assets: Archive Artifact + Checksum Artifact
   ↓
8. Release published (available to users)
```

## File System Artifacts

These are the concrete files produced during the workflow:

```text
target/aarch64-apple-darwin/release/
├── autofix                           # Binary Artifact (Mach-O executable)
├── autofix-macos-aarch64.zip         # Archive Artifact
└── autofix-macos-aarch64.zip.sha256  # Checksum Artifact

# Ephemeral (created and used, not preserved):
RELEASE_NOTES.md                      # Build Metadata template
```

## State Transitions

### Release Tag States

1. **Not Created**: No tag exists
2. **Created**: Tag pushed to repository
3. **Workflow Triggered**: GitHub Actions workflow started
4. **Workflow Complete**: All artifacts built successfully

(Tags themselves don't have states, but we track the workflow state per tag)

### GitHub Release States

1. **Non-existent**: No release for this tag
2. **Creating**: Workflow is running, release not yet created
3. **Published**: Release created with artifacts attached
4. **Error**: Workflow failed, no release created (terminal state for this run)

Notably, there are no draft states - releases are immediately published per FR specification.

## Notes

- This feature has no traditional CRUD operations or API endpoints
- No database storage is involved (GitHub manages release data)
- Artifacts are immutable once created (no updates or deletes via workflow)
- The entire data model is file-based and event-driven (tag push → workflow → release)
