---
description: "Implementation tasks for automated release build artifacts"
---

# Tasks: Release Build Artifacts

**Input**: Design documents from `/specs/002-release-build-artifacts/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/workflow-contract.md

**Tests**: Not requested in this feature specification - workflow verification is manual per quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions

This is CI/CD infrastructure, not application code. Primary artifact is `.github/workflows/release.yml`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create necessary directory structure for GitHub Actions workflow

- [ ] T001 Create .github/workflows directory if it doesn't exist
- [ ] T002 Review research.md decisions for workflow implementation details

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core workflow structure that MUST be complete before adding features

**⚠️ CRITICAL**: User Story 1 (P1) depends on this foundation

- [ ] T003 Create basic workflow file at .github/workflows/release.yml with name and trigger structure
- [ ] T004 Configure workflow trigger for tag pattern v*.*.* in .github/workflows/release.yml
- [ ] T005 Add checkout step using actions/checkout@v4 in .github/workflows/release.yml
- [ ] T006 Add Rust toolchain setup step in .github/workflows/release.yml

**Checkpoint**: Foundation ready - workflow can trigger on tags and has build environment

---

## Phase 3: User Story 1 - Automated Release Creation (Priority: P1) 🎯 MVP

**Goal**: When a maintainer creates a version tag (v*.*.*), a GitHub Actions workflow automatically builds the autofix binary for Apple Silicon, creates a GitHub release, and attaches the zipped binary

**Independent Test**: Create a test tag (e.g., v0.0.1-test), verify workflow runs successfully, check that a GitHub release is created with downloadable zip file, download and run the binary on an Apple Silicon Mac

### Implementation for User Story 1

- [ ] T007 [US1] Add build step using cargo build --release --target aarch64-apple-darwin in .github/workflows/release.yml
- [ ] T008 [US1] Add binary packaging step to create zip archive in .github/workflows/release.yml
- [ ] T009 [US1] Add step to generate RELEASE_NOTES.md with build metadata in .github/workflows/release.yml
- [ ] T010 [US1] Add GitHub release creation step using softprops/action-gh-release@v1 in .github/workflows/release.yml
- [ ] T011 [US1] Configure release step to upload autofix-macos-aarch64.zip in .github/workflows/release.yml
- [ ] T012 [US1] Configure release step to use RELEASE_NOTES.md as body_path in .github/workflows/release.yml
- [ ] T013 [US1] Set draft: false and prerelease: false in release configuration in .github/workflows/release.yml
- [ ] T014 [US1] Add build metadata collection (commit SHA, Rust version, build date) to RELEASE_NOTES.md generation in .github/workflows/release.yml

**Checkpoint**: At this point, User Story 1 should be fully functional - tagging a version should create a release with a working binary

---

## Phase 4: User Story 2 - Release Artifact Verification (Priority: P2)

**Goal**: Users can verify downloaded binaries are authentic by checking SHA256 checksums published in release notes

**Independent Test**: Download a release artifact and its checksum file, run `shasum -a 256 -c autofix-macos-aarch64.zip.sha256`, verify output shows "OK"

### Implementation for User Story 2

- [ ] T015 [US2] Add checksum generation step using shasum -a 256 after zip creation in .github/workflows/release.yml
- [ ] T016 [US2] Add checksum file to release assets upload list in .github/workflows/release.yml
- [ ] T017 [US2] Update RELEASE_NOTES.md template to include checksum in dedicated section in .github/workflows/release.yml

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - releases include checksums that users can verify

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Validation and documentation for the release workflow

- [ ] T018 [P] Manual test: Create test tag v0.0.1-test and verify workflow completes successfully
- [ ] T019 [P] Manual test: Download artifacts from test release and verify checksum
- [ ] T020 [P] Manual test: Extract and run binary on Apple Silicon Mac to verify it executes
- [ ] T021 [P] Manual test: Verify release notes contain all required build metadata
- [ ] T022 [P] Manual test: Verify workflow fails gracefully if build errors occur (test with broken code)
- [ ] T023 [P] Manual test: Verify workflow only triggers on v*.*.* tags, not other tags or branches
- [ ] T024 Clean up test release v0.0.1-test after successful validation
- [ ] T025 Update CLAUDE.md if any new technologies were added (already done by update-agent-context.sh)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User Story 1 (P1): Must complete before User Story 2
  - User Story 2 (P2): Enhances User Story 1 (adds checksums to existing release process)
- **Polish (Phase 5)**: Depends on both user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories - This is the MVP
- **User Story 2 (P2)**: Can start after User Story 1 is complete - Adds checksum generation to existing workflow

### Within Each User Story

**User Story 1**:
- Build step (T007) must complete before packaging (T008)
- Packaging (T008) must complete before release notes generation (T009)
- All artifacts must be ready before release creation (T010)
- Release creation configuration (T011-T014) can be done in any order

**User Story 2**:
- Checksum generation (T015) must happen after T008 (zip creation from US1)
- Checksum file upload (T016) and release notes update (T017) can be done in parallel

### Parallel Opportunities

- **Phase 1**: Both tasks can run in parallel (different directories)
- **Phase 2**: Tasks T003-T006 are sequential (all edit same file, build on each other)
- **User Story 1**: Tasks T011-T014 can run in parallel (configure different aspects of release)
- **User Story 2**: Tasks T016 and T017 can run in parallel (different parts of workflow)
- **Polish**: All manual tests (T018-T023) can run in parallel if multiple testers available

---

## Parallel Example: User Story 1

```bash
# Sequential dependency chain for core workflow:
# T007 (build) → T008 (package) → T009 (release notes) → T010 (release creation)

# Then configure release in parallel:
Task: "Configure release step to upload autofix-macos-aarch64.zip in .github/workflows/release.yml" [T011]
Task: "Configure release step to use RELEASE_NOTES.md as body_path in .github/workflows/release.yml" [T012]
Task: "Set draft: false and prerelease: false in release configuration in .github/workflows/release.yml" [T013]
Task: "Add build metadata collection to RELEASE_NOTES.md generation in .github/workflows/release.yml" [T014]
```

---

## Parallel Example: Polish Phase

```bash
# All validation tests can run in parallel with multiple team members:
Task: "Manual test: Create test tag and verify workflow completes" [T018]
Task: "Manual test: Download and verify checksum" [T019]
Task: "Manual test: Extract and run binary on Apple Silicon" [T020]
Task: "Manual test: Verify release notes contain metadata" [T021]
Task: "Manual test: Verify workflow fails gracefully with errors" [T022]
Task: "Manual test: Verify workflow only triggers on correct tags" [T023]
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: Foundational (T003-T006) - Foundation ready
3. Complete Phase 3: User Story 1 (T007-T014)
4. **STOP and VALIDATE**: Create a test tag, verify release is created with working binary
5. MVP is complete - automated releases work!

### Incremental Delivery

1. Complete Setup + Foundational → Workflow structure ready
2. Add User Story 1 → Test with v0.0.1-test tag → **MVP delivered!** (Automated releases work)
3. Add User Story 2 → Test checksum verification → Enhanced security validation
4. Complete Polish → Full validation and cleanup
5. Ready for production use with first real release tag (e.g., v1.0.0)

### Sequential Implementation (Recommended for Single Developer)

Since this is a single workflow file with interdependent steps:

1. Phase 1: Setup (5 minutes)
2. Phase 2: Foundational (15 minutes)
3. Phase 3: User Story 1 (30 minutes)
   - Test immediately with v0.0.1-test
4. Phase 4: User Story 2 (10 minutes)
   - Test checksum verification
5. Phase 5: Polish (20 minutes for all validation tests)

**Total estimated time**: ~80 minutes for complete implementation and validation

---

## Notes

- This feature has minimal [P] parallelization opportunities because all tasks edit the same workflow file
- Most tasks are sequential due to logical dependencies in the workflow steps
- The workflow itself is designed for automation - manual testing is only during initial setup
- Once deployed, the workflow runs automatically on every version tag push
- **Critical**: Test with a non-production tag (v0.0.1-test) before creating your first real release
- Remember to clean up test releases after validation
- Workflow failures will be visible in GitHub Actions tab - no partial releases will be created
- Binary must be tested on actual Apple Silicon hardware to verify compatibility

---

## Success Criteria Validation

After completing all tasks, verify against spec.md Success Criteria:

- **SC-001**: ✅ When a version tag is pushed, release created within 10 minutes with binary attached
- **SC-002**: ✅ Downloaded binaries execute successfully on Apple Silicon Macs without requiring Rust
- **SC-003**: ✅ 100% of v*.*.* tags result in successful release creation or documented failure
- **SC-004**: ✅ Users can download and run binary in under 3 minutes from release page
- **SC-005**: ✅ Release artifacts include verification checksums for integrity validation

---

## File Summary

**Single File Modified**:
- `.github/workflows/release.yml` (created) - All 25 tasks contribute to this one file

**Artifacts Generated by Workflow** (not in repository):
- `target/aarch64-apple-darwin/release/autofix` (binary)
- `target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip` (archive)
- `target/aarch64-apple-darwin/release/autofix-macos-aarch64.zip.sha256` (checksum)
- `RELEASE_NOTES.md` (ephemeral, used for release body)

**Testing Artifacts**:
- Test release: v0.0.1-test (created in T018, deleted in T024)
