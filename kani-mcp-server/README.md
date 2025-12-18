# Amazon Q CLI with Kani MCP Server

Complete guide for setting up and using Amazon Q CLI with the Kani verification MCP server.

## Table of Contents

- [Installing Amazon Q CLI](#installing-amazon-q-cli)
- [Setting Up Your AWS Builder ID](#setting-up-your-aws-builder-id)
- [Installing the Kani MCP Server](#installing-the-kani-mcp-server)
- [Configuring Amazon Q CLI](#configuring-amazon-q-cli)
- [Using Amazon Q with Kani](#using-amazon-q-with-kani)
- [Troubleshooting](#troubleshooting)
- [Quick Reference](#quick-reference)

---

## Installing Amazon Q CLI

### Prerequisites

- macOS
- Internet connection
- Homebrew (recommended)

### Installation Methods

#### Option 1: Homebrew (Recommended)

```bash
# Install Amazon Q CLI
brew install --cask amazon-q

# Verify installation
q --version
```

#### Option 2: Direct Download

1. Download from the official AWS documentation
2. Double-click the .dmg file
3. Drag Amazon Q to Applications folder
4. Open Amazon Q from Applications
5. Enable shell integrations when prompted

### Post-Installation Setup

After installation, restart your terminal and verify:

```bash
# Check if q command is available
which q

# Check version
q --version

# Run diagnostics
q doctor
```

---

## Setting Up Your AWS Builder ID

Amazon Q CLI requires a free AWS Builder ID for authentication.

### Step 1: Login

```bash
q login
```

This will:
- Open your web browser automatically
- Direct you to the AWS Builder ID login page
- Ask you to either sign in or create a new Builder ID

### Step 2: Create Builder ID (First Time Users)

If you don't have a Builder ID:

1. Click "Create AWS Builder ID" on the login page
2. Enter your email address
3. Choose a display name
4. Verify your email (check your inbox)
5. Complete the registration

### Step 3: Authorize Amazon Q

1. After logging in, authorize Amazon Q Developer
2. Return to your terminal
3. You should see a success message

### Verification

```bash
# Check authentication status
q doctor

# You should see:
# ✔ Everything looks good!
```

---

## Installing the Kani MCP Server

### Prerequisites

Install [Kani Rust Verifier](https://model-checking.github.io/kani/) - a model checker for Rust:

```bash
# Install Kani
cargo install --locked kani-verifier
cargo kani setup

# Verify installation
cargo kani --version
```

### Clone and Build Kani MCP Server

```bash
# Clone the repository
git clone <your-kani-mcp-server-repo>
cd kani-mcp-server

# Build the server
cargo build --release

# Verify the binary exists
ls -lh target/release/kani-mcp-server
```

The binary will be at: `target/release/kani-mcp-server`

---

## Configuring Amazon Q CLI

### Step 1: Create MCP Configuration Directory

```bash
mkdir -p ~/.aws/amazonq
```

### Step 2: Create MCP Configuration File

Create `~/.aws/amazonq/mcp.json`:

```bash
cat > ~/.aws/amazonq/mcp.json << 'EOF'
{
  "mcpServers": {
    "kani-verifier": {
      "command": "/absolute/path/to/kani-mcp-server",
      "env": {},
      "disabled": false,
      "autoApprove": []
    }
  }
}
EOF
```

**Important:** Replace `/absolute/path/to/kani-mcp-server` with your actual path, for example:
- `/Users/yourusername/target/release/kani-mcp-server`

### Step 3: Verify Configuration

```bash
# Check the config file exists
cat ~/.aws/amazonq/mcp.json

# Verify the path is correct
ls -lh /absolute/path/to/kani-mcp-server
```

### Step 4: Test MCP Server Loading

```bash
# Start Amazon Q CLI
q chat

# You should see:
# ✓ kani-verifier loaded in 0.0X s
```

If you see this message, your MCP server is successfully configured! ✅

---

## Using Amazon Q with Kani

### Starting a Chat Session

```bash
q chat
```

### Basic Commands

Inside the Q chat interface:

- **Send a message:** Just type and press Enter
- **Multi-line input:** Press `Ctrl + J` for new lines
- **Exit:** Type `/q` or press `Ctrl + C`
- **Help:** Type `/help`
- **Clear screen:** Type `/clear`

### Checking Available Tools

Once in the chat, verify your Kani tools are loaded:

```
What MCP tools do you have?
```

You should see:
- `verify_rust_project` - Run Kani verification on a Rust project
- `verify_unsafe_code` - Verify unsafe Rust code blocks
- `explain_failure` - Analyze verification failures
- `generate_kani_harness` - Generate proof harness templates

### Example Usage Scenarios

#### 1. Verify a Rust Project

```
I have a Rust project at ~/my-project. Can you verify it with Kani?
```

Or more specifically:

```
Run Kani verification on the project at /Users/username/my-project 
with the harness function verify::check_bounds
```

#### 2. Generate a Kani Harness

```
Generate a Kani harness for a function called `add` that takes two u32 
parameters and returns their sum. Check for overflow.
```

#### 3. Verify Unsafe Code

```
I have unsafe code in ~/my-project/src/lib.rs. Can you verify the 
memory safety using the harness verify::check_unsafe_ptr?
```

#### 4. Explain Verification Failures

After running a verification that fails:

```
Can you explain why the last Kani verification failed?
```

Or paste the output:

```
Explain this Kani verification failure:
[paste Kani output here]
```

### Real-World Example

```
> I'm working on a binary search function. Can you generate a Kani 
  harness to verify it never panics and always returns the correct index?

> Here's my code:
  fn binary_search<T: Ord>(arr: &[T], target: &T) -> Option<usize> {
      let mut low = 0;
      let mut high = arr.len();
      
      while low < high {
          let mid = (low + high) / 2;
          match arr[mid].cmp(target) {
              std::cmp::Ordering::Equal => return Some(mid),
              std::cmp::Ordering::Less => low = mid + 1,
              std::cmp::Ordering::Greater => high = mid,
          }
      }
      None
  }
```

Amazon Q will then:
1. Generate a Kani proof harness
2. Suggest properties to verify
3. Help you run the verification
4. Explain any failures found

---

## Troubleshooting

### Common Issues and Solutions

#### 1. "command not found: q"

**Problem:** Amazon Q CLI is not in your PATH

**Solution:**

```bash
# Reload your shell configuration
source ~/.zshrc  # or ~/.bashrc

# Or restart your terminal
```

#### 2. "kani-verifier has failed to load"

**Problem:** MCP server configuration is incorrect

**Solutions:**

a) Check the server binary exists:
```bash
ls -lh /path/to/kani-mcp-server
```

b) Test the server manually:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | /path/to/kani-mcp-server
```

c) Check MCP configuration:
```bash
cat ~/.aws/amazonq/mcp.json
```

d) Verify the path in the config matches the actual binary location:
```bash
# Get the absolute path (assuming you're in the kani-mcp-server directory)
pwd
realpath target/release/kani-mcp-server

# Update config with this path
```

#### 3. "Broken pipe" errors in logs

**Problem:** Server logging is causing issues

**Solution:** This should already be fixed in the latest version, but if you see it:

```bash
# Rebuild with the fixed version (in your kani-mcp-server directory)
cargo build --release

# The fixed version disables logging that causes broken pipes
```

#### 4. Server starts but tools don't appear

**Problem:** MCP protocol handshake issue

**Solution:**

```bash
# Check if the binary is the latest version
ls -lh target/release/kani-mcp-server

# Rebuild if necessary (in your kani-mcp-server directory)
cargo build --release

# Restart Q
q restart
q chat
```

#### 5. "Authentication failed"

**Problem:** Builder ID session expired

**Solution:**

```bash
# Re-authenticate
q login

# Follow the browser prompts
```

### Debug Mode

Enable detailed logging to diagnose issues:

```bash
# Run with trace logging
Q_LOG_LEVEL=trace q chat

# Check logs
# macOS: /var/folders/*/T/qlog/qchat.log

# View recent logs
tail -f /var/folders/*/T/qlog/qchat.log
```

### Getting Help

```bash
# Run diagnostics
q doctor

# Report issues
q issue

# View help
q --help
q chat --help
```

---

## Quick Reference

### Essential Commands

```bash
# Installation & Setup
brew install --cask amazon-q       # Install
q login                             # Authenticate
q doctor                            # Check status

# Chat Interface
q chat                              # Start chat
/q or Ctrl+C                        # Exit chat
/help                               # Show help
/clear                              # Clear screen
Ctrl+J                              # New line (multi-line input)

# Configuration
~/.aws/amazonq/mcp.json            # MCP config file
q restart                           # Restart Q daemon
q issue                             # Report bug

# Inline Completions
q inline enable                     # Enable autocomplete
q inline disable                    # Disable autocomplete
```

### Directory Structure

```
~/.aws/amazonq/
├── mcp.json              # MCP server configuration
├── profiles/             # Profile configurations
└── cache/                # Cached data

/var/folders/*/T/qlog/    # Log files
└── qchat.log            # Chat session logs
```

### Testing Your Setup

```bash
# 1. Test Q CLI
q --version

# 2. Test authentication
q doctor

# 3. Test MCP server manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | /path/to/kani-mcp-server

# 4. Test with Q
q chat
# Then ask: "What MCP tools do you have?"
```

