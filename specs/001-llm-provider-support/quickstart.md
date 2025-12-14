# Quickstart: LLM Provider Support

**Date**: 2025-12-12
**Feature**: 001-llm-provider-support

## Overview

This guide helps you configure and use Autofix with different LLM providers (Claude, OpenAI, or Ollama).

## Prerequisites

- Autofix installed (`cargo build --release`)
- macOS with Xcode command-line tools
- API keys or local model setup (depending on provider)

## Provider Comparison

| Provider | Cost | Privacy | Offline | Setup Complexity | Model Quality |
|----------|------|---------|---------|------------------|---------------|
| **Claude** | $$ | Cloud | No | Easy | Excellent |
| **OpenAI** | $$$ | Cloud | No | Easy | Excellent |
| **Ollama** | Free | Local | Yes | Medium | Good |

---

## Option 1: Anthropic Claude (Default)

**Best for**: Production use, complex test fixing, highest quality results

### Setup Steps

1. **Get API Key**:
   ```bash
   # Sign up at https://console.anthropic.com
   # Navigate to API Keys section
   # Create new key
   ```

2. **Set Environment Variable**:
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

3. **Verify Configuration**:
   ```bash
   autofix --version
   # Should show: autofix 0.1.0 (provider: claude)
   ```

4. **Run Autofix**:
   ```bash
   autofix fix \
     --test-result path/to/test.xcresult \
     --workspace path/to/workspace
   ```

### Configuration Options

```bash
# Use Claude Haiku for faster/cheaper responses
export AUTOFIX_MODEL="claude-haiku-3.5"

# Adjust rate limiting (default: 30000 TPM)
export AUTOFIX_RATE_LIMIT_TPM=50000

# Increase timeout for complex operations
export AUTOFIX_TIMEOUT_SECS=60
```

### Troubleshooting

**Error: "Authentication failed"**
- Check API key is correct: `echo $ANTHROPIC_API_KEY`
- Verify key starts with `sk-ant-`
- Ensure no extra whitespace in key

**Error: "Rate limit exceeded"**
- Reduce `AUTOFIX_RATE_LIMIT_TPM` to match your tier
- Add delays between requests
- Upgrade Anthropic account tier

---

## Option 2: OpenAI

**Best for**: Users with existing OpenAI credits, alternative to Claude

### Setup Steps

1. **Get API Key**:
   ```bash
   # Sign up at https://platform.openai.com
   # Navigate to API keys
   # Create new secret key
   ```

2. **Set Environment Variables**:
   ```bash
   export OPENAI_API_KEY="sk-..."
   export AUTOFIX_PROVIDER="openai"
   ```

3. **Run Autofix**:
   ```bash
   autofix fix \
     --provider openai \
     --test-result path/to/test.xcresult \
     --workspace path/to/workspace
   ```

### Configuration Options

```bash
# Use GPT-4 Turbo (default: gpt-4)
export AUTOFIX_MODEL="gpt-4-turbo"

# Adjust rate limiting (default: 90000 TPM)
export AUTOFIX_RATE_LIMIT_TPM=200000

# Use custom OpenAI-compatible endpoint
export AUTOFIX_API_BASE="https://api.together.xyz/v1"
export OPENAI_API_KEY="your-together-api-key"
```

### OpenAI-Compatible Providers

Autofix works with any OpenAI-compatible API:

**Together.ai**:
```bash
export AUTOFIX_PROVIDER="openai"
export AUTOFIX_API_BASE="https://api.together.xyz/v1"
export OPENAI_API_KEY="your-together-key"
export AUTOFIX_MODEL="meta-llama/Llama-3-70b-chat-hf"
```

**Groq**:
```bash
export AUTOFIX_PROVIDER="openai"
export AUTOFIX_API_BASE="https://api.groq.com/openai/v1"
export OPENAI_API_KEY="your-groq-key"
export AUTOFIX_MODEL="llama-3.1-70b-versatile"
```

**Azure OpenAI**:
```bash
export AUTOFIX_PROVIDER="openai"
export AUTOFIX_API_BASE="https://your-resource.openai.azure.com/openai/deployments/your-deployment"
export OPENAI_API_KEY="your-azure-key"
```

### Troubleshooting

**Error: "Invalid API key"**
- Verify key: `echo $OPENAI_API_KEY`
- Check key starts with `sk-`
- Ensure API base URL is correct

**Error: "Model not found"**
- List available models: `curl https://api.openai.com/v1/models -H "Authorization: Bearer $OPENAI_API_KEY"`
- Use exact model name from list

---

## Option 3: Ollama (Local)

**Best for**: Offline use, privacy-sensitive code, zero API costs

### Setup Steps

1. **Install Ollama**:
   ```bash
   # macOS
   brew install ollama

   # Or download from https://ollama.ai
   ```

2. **Download Models**:
   ```bash
   # Start Ollama service
   ollama serve &

   # Pull recommended model
   ollama pull llama2

   # Or use more capable models
   ollama pull codellama     # Better for code
   ollama pull mistral       # Larger context window
   ```

3. **Verify Installation**:
   ```bash
   # List downloaded models
   ollama list

   # Should show:
   # NAME           ID          SIZE    MODIFIED
   # llama2:latest  78e26419b4  3.8 GB  2 minutes ago
   ```

4. **Set Environment Variables**:
   ```bash
   export AUTOFIX_PROVIDER="ollama"
   export AUTOFIX_MODEL="llama2"  # Or codellama, mistral, etc.
   ```

5. **Run Autofix (Offline)**:
   ```bash
   # Disconnect from internet (optional - proves offline capability)
   # networksetup -setairportpower en0 off

   autofix fix \
     --provider ollama \
     --test-result path/to/test.xcresult \
     --workspace path/to/workspace
   ```

### Configuration Options

```bash
# Use custom Ollama endpoint
export AUTOFIX_API_BASE="http://localhost:11434/v1"

# No rate limiting needed (local)
export AUTOFIX_RATE_LIMIT_TPM=0  # Unlimited

# Increase timeout for larger models
export AUTOFIX_TIMEOUT_SECS=120
```

### Model Recommendations

| Model | Size | Context | Best For |
|-------|------|---------|----------|
| **llama2** | 3.8 GB | 4K tokens | General use, fast |
| **codellama** | 3.8 GB | 16K tokens | Code-specific tasks |
| **mistral** | 4.1 GB | 32K tokens | Larger context needed |
| **llama3:70b** | 40 GB | 8K tokens | Best quality (needs powerful Mac) |

### Troubleshooting

**Error: "Connection refused"**
- Start Ollama service: `ollama serve`
- Verify running: `curl http://localhost:11434/v1/models`
- Check port not blocked by firewall

**Error: "Model not found"**
- List available models: `ollama list`
- Pull missing model: `ollama pull llama2`
- Use exact model name

**Error: "Out of memory"**
- Use smaller model: `llama2` instead of `llama3:70b`
- Close other applications
- Check system RAM: `top -l 1 | grep PhysMem`

**Performance is slow**
- Ensure Ollama using GPU acceleration (Metal on macOS)
- Reduce context length: `export AUTOFIX_MAX_TOKENS=2000`
- Use faster model: `llama2` instead of `mistral`

---

## Switching Between Providers

You can switch providers anytime without changing code:

### Using Environment Variables

```bash
# Use Claude
export AUTOFIX_PROVIDER="claude"
export ANTHROPIC_API_KEY="sk-ant-..."
autofix fix --test-result ...

# Switch to OpenAI
export AUTOFIX_PROVIDER="openai"
export OPENAI_API_KEY="sk-..."
autofix fix --test-result ...

# Switch to Ollama
export AUTOFIX_PROVIDER="ollama"
autofix fix --test-result ...
```

### Using CLI Flags

```bash
# Claude (default)
autofix fix --test-result test.xcresult --workspace .

# OpenAI
autofix fix --provider openai --test-result test.xcresult --workspace .

# Ollama
autofix fix --provider ollama --model codellama --test-result test.xcresult --workspace .
```

### Using Configuration File (.env)

```bash
# Create .env file
cat > .env <<EOF
AUTOFIX_PROVIDER=claude
ANTHROPIC_API_KEY=sk-ant-...
AUTOFIX_MODEL=claude-sonnet-4
AUTOFIX_RATE_LIMIT_TPM=30000
EOF

# Autofix automatically loads .env
autofix fix --test-result test.xcresult --workspace .
```

---

## Cost Comparison

### Per 1000 Test Fixes (Estimated)

| Provider | Model | Input Cost | Output Cost | Total Est. |
|----------|-------|------------|-------------|------------|
| Claude | Sonnet 4 | $3.00/MTok | $15.00/MTok | ~$50-100 |
| Claude | Haiku 3.5 | $0.80/MTok | $4.00/MTok | ~$15-30 |
| OpenAI | GPT-4 Turbo | $10.00/MTok | $30.00/MTok | ~$150-300 |
| OpenAI | GPT-4 | $30.00/MTok | $60.00/MTok | ~$400-600 |
| Ollama | Any | $0 | $0 | **Free** |

*Estimates based on average token usage per test fix (~3K input, ~1K output)*

---

## Advanced Configuration

### Rate Limiting

```bash
# Conservative (avoid throttling)
export AUTOFIX_RATE_LIMIT_TPM=10000

# Aggressive (faster, risk throttling)
export AUTOFIX_RATE_LIMIT_TPM=100000

# Disable (local Ollama)
export AUTOFIX_RATE_LIMIT_TPM=0
```

### Retry Configuration

```bash
# More retries for unreliable networks
export AUTOFIX_MAX_RETRIES=5

# Faster failure for stable networks
export AUTOFIX_MAX_RETRIES=1
```

### Timeout Configuration

```bash
# Quick timeout for fast models
export AUTOFIX_TIMEOUT_SECS=15

# Long timeout for complex fixes
export AUTOFIX_TIMEOUT_SECS=120
```

### Verbose Logging

```bash
# See provider details and token usage
autofix fix --verbose --test-result ...

# Output includes:
# - Provider type and model
# - Token counts per request
# - Rate limit status
# - Retry attempts
# - Response times
```

---

## Recommendation by Use Case

### "I want the best quality fixes"
→ **Use Claude Sonnet 4** (default provider)

### "I want to minimize costs"
→ **Use Claude Haiku 3.5** or **Ollama (free)**

### "I need to work offline"
→ **Use Ollama** (fully local)

### "I have existing OpenAI credits"
→ **Use OpenAI** with API base configuration

### "I'm working with proprietary code"
→ **Use Ollama** (data never leaves your machine)

---

## Next Steps

- Read the full documentation: `docs/llm-providers.md`
- Configure your preferred provider (see setup above)
- Run your first test fix
- Fine-tune rate limits and timeouts for your workflow

## Support

- Issues: https://github.com/your-repo/autofix/issues
- Discussions: https://github.com/your-repo/autofix/discussions
- Anthropic API docs: https://docs.anthropic.com
- OpenAI API docs: https://platform.openai.com/docs
- Ollama docs: https://ollama.ai/docs
