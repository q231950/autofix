Key Distinction:

| Mode | Assumption | Primary Target | Can Modify App? | Can Modify Test? |
|------|------------|----------------|-----------------|------------------|
| **Knight Rider** (`--knightrider`) | Test is correct | Fix app code | ✅ Yes (only this) | ❌ No |
| **Standard** (default) | App is correct | Fix test code | ✅ Yes (accessibility only) | ✅ Yes (suggestions) |

Usage:

**Without `--knightrider` (Standard Mode):**
```bash
autofix --ios --test-result path.xcresult --workspace path/to/workspace
```
- Will use tools to fix **test code** and add **accessibility identifiers to app code**
- Assumes app is correct, test needs adjustment

**With `--knightrider` (Knight Rider Mode):**
```bash
autofix --ios --test-result path.xcresult --workspace path/to/workspace --knightrider
```
- Will use tools to fix **app code only**
- Assumes test is correct, app needs to match expectations
