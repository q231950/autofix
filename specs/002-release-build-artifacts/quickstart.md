# Quickstart: Release Build Artifacts

**Feature**: 002-release-build-artifacts
**Date**: 2025-12-14

## Overview

This quickstart guide shows how to use the automated release system to publish new versions of autofix with precompiled Apple Silicon binaries.

## Prerequisites

- Maintainer access to the autofix repository (write permissions)
- Git configured with push access
- GitHub Actions enabled on the repository
- Local clone of the repository

## Creating a Release (Maintainer)

### Step 1: Prepare the Release

Ensure all changes are merged to the main branch and ready for release.

```bash
# Update local main branch
git checkout main
git pull origin main

# Verify build works locally
cargo build --release
cargo test
```

### Step 2: Create and Push a Version Tag

Choose a semantic version number (e.g., v1.0.0, v1.2.3).

```bash
# Create an annotated tag
git tag -a v1.0.0 -m "Release version 1.0.0"

# Push the tag to GitHub
git push origin v1.0.0
```

**Important**: Only tags matching the pattern `v*.*.*` will trigger releases.

### Step 3: Monitor the Workflow

1. Go to the repository on GitHub
2. Click the "Actions" tab
3. Find the workflow run for your tag
4. Monitor progress (typical duration: 3-5 minutes)

**Workflow URL**: `https://github.com/<owner>/autofix/actions`

### Step 4: Verify the Release

Once the workflow completes:

1. Go to the "Releases" page: `https://github.com/<owner>/autofix/releases`
2. Find your release (tagged with your version)
3. Verify the release contains:
   - `autofix-macos-aarch64.zip` (binary archive)
   - `autofix-macos-aarch64.zip.sha256` (checksum)
   - Release notes with build metadata

## Downloading a Release (User)

### Step 1: Find the Latest Release

Visit the releases page: `https://github.com/<owner>/autofix/releases`

Click on the latest release (or the specific version you want).

### Step 2: Download the Binary

Click on `autofix-macos-aarch64.zip` to download.

**Optional**: Also download `autofix-macos-aarch64.zip.sha256` for verification.

### Step 3: Verify the Download (Recommended)

```bash
# Navigate to your downloads folder
cd ~/Downloads

# Verify the checksum
shasum -a 256 -c autofix-macos-aarch64.zip.sha256
```

Expected output:
```
autofix-macos-aarch64.zip: OK
```

### Step 4: Extract and Install

```bash
# Extract the archive
unzip autofix-macos-aarch64.zip

# Move to a directory in your PATH
mv autofix /usr/local/bin/

# Verify the installation
autofix --version
```

**Note**: On first run, macOS may show a security warning. If this happens:
1. Go to System Preferences → Security & Privacy
2. Click "Allow Anyway" for autofix
3. Run `autofix --version` again
4. Click "Open" when prompted

## Troubleshooting

### Workflow Fails to Trigger

**Symptom**: You pushed a tag but no workflow runs appear.

**Solutions**:
- Verify tag matches pattern `v*.*.*` (check with `git tag -l`)
- Ensure GitHub Actions are enabled in repository settings
- Check the workflow file exists at `.github/workflows/release.yml`

### Build Fails During Workflow

**Symptom**: Workflow runs but fails at the build step.

**Solutions**:
- Check the workflow logs for specific error messages
- Verify the code builds locally: `cargo build --release`
- Ensure all tests pass: `cargo test`
- Fix any issues and create a new tag (e.g., v1.0.1)

### Release Already Exists

**Symptom**: Workflow updates an existing release instead of failing.

**Explanation**: The `softprops/action-gh-release` action handles duplicate tags by updating the existing release. This is intentional.

**Solutions**:
- If you want a fresh release, delete the existing release on GitHub first
- Or use a new tag version (e.g., v1.0.1 instead of v1.0.0)

### Binary Won't Run on Apple Silicon Mac

**Symptom**: Downloaded binary shows "cannot execute binary file" or similar error.

**Solutions**:
- Verify you downloaded the correct file (`autofix-macos-aarch64.zip`)
- Verify checksum to ensure download wasn't corrupted
- Check macOS version (requires macOS 11.0 or later)
- Ensure you're on Apple Silicon (M1/M2/M3): `uname -m` should show `arm64`

### Checksum Verification Fails

**Symptom**: `shasum -a 256 -c` reports checksum doesn't match.

**Solutions**:
- Re-download the zip file (download may have been corrupted)
- Ensure you downloaded the matching `.sha256` file from the same release
- Verify file integrity: try downloading from a different network

## Advanced Usage

### Testing the Workflow Without Public Release

Create a prerelease tag to test:

```bash
# Create a prerelease tag
git tag v0.1.0-beta
git push origin v0.1.0-beta
```

**Note**: Current workflow configuration publishes all releases. To mark as prerelease, modify the workflow file (see contracts/workflow-contract.md).

### Deleting a Release

If you need to remove a bad release:

1. Go to the releases page on GitHub
2. Click the release you want to delete
3. Click "Delete" button
4. Confirm deletion

**Note**: This doesn't delete the git tag. To delete the tag:

```bash
# Delete local tag
git tag -d v1.0.0

# Delete remote tag
git push origin :refs/tags/v1.0.0
```

### Creating a Release from a Specific Commit

By default, tags are created from the current HEAD. To tag a specific commit:

```bash
# Tag a specific commit
git tag -a v1.0.0 <commit-sha> -m "Release 1.0.0"
git push origin v1.0.0
```

## Expected Workflow Timeline

| Step | Duration | Cumulative |
|------|----------|------------|
| Checkout code | 10-30s | 0:30 |
| Setup Rust toolchain | 30-60s | 1:30 |
| Build binary | 2-4 min | 5:30 |
| Package & checksum | 5-10s | 5:40 |
| Create release | 30-60s | 6:30 |
| **Total** | **~3-7 min** | **~6:30** |

## Quick Reference

### Create Release (One Command)

```bash
# Create and push tag in one step
git tag -a v1.0.0 -m "Release 1.0.0" && git push origin v1.0.0
```

### Download and Install (User)

```bash
# Download, verify, extract, and install
cd ~/Downloads
curl -LO https://github.com/<owner>/autofix/releases/download/v1.0.0/autofix-macos-aarch64.zip
curl -LO https://github.com/<owner>/autofix/releases/download/v1.0.0/autofix-macos-aarch64.zip.sha256
shasum -a 256 -c autofix-macos-aarch64.zip.sha256
unzip autofix-macos-aarch64.zip
sudo mv autofix /usr/local/bin/
autofix --version
```

## Related Documentation

- [Workflow Contract](./contracts/workflow-contract.md) - Technical specification of the workflow
- [Data Model](./data-model.md) - Artifact and release entity descriptions
- [Research](./research.md) - Technology decisions and rationale

## Support

For issues with the release process:
1. Check the GitHub Actions workflow logs
2. Review the troubleshooting section above
3. Open an issue on the repository with workflow logs attached
