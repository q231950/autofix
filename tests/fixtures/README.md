# Test Fixtures

This directory contains test fixtures for the autofix project.

## test.xcresult

A placeholder directory for an Xcode test result bundle. Replace with a real `.xcresult` bundle to enable integration tests.

### How to generate a real .xcresult file:

```bash
# Run tests and generate xcresult bundle
xcodebuild test \
  -scheme YourScheme \
  -destination 'platform=iOS Simulator,name=iPhone 15' \
  -resultBundlePath ./test.xcresult

# Copy to fixtures directory
cp -r ./test.xcresult tests/fixtures/
```

### Alternative: Use xcresulttool to export JSON

If you have an existing .xcresult file, you can view its structure:

```bash
xcrun xcresulttool get --format json --path test.xcresult
```

This will help you understand the JSON structure that the parser expects.
