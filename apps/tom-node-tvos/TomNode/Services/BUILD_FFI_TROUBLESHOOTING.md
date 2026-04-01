# Troubleshooting Guide: build_ffi.sh

## Common Causes of "Command PhaseScriptExecution failed" Error

### 1. **Rust Not Installed or Not in PATH**

**Symptoms:**
- Error message: "Cargo n'est pas installé"
- Build fails immediately

**Solution:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH (add to ~/.zshrc or ~/.bash_profile)
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell or source the profile
source ~/.zshrc
```

### 2. **tvOS Target Not Installed**

**Symptoms:**
- Error message about missing target
- "Impossible d'installer la cible aarch64-apple-tvos"

**Solution:**
```bash
# Install tvOS targets
rustup target add aarch64-apple-tvos
rustup target add aarch64-apple-tvos-sim
```

### 3. **Workspace Not Found**

**Symptoms:**
- "Impossible de trouver le workspace Cargo avec tom-protocol-ffi"
- Build script can't locate the Rust project

**Solution:**
- Verify the path to your Rust workspace
- Update the hardcoded path in the script if needed:
  ```bash
  # Edit build_ffi.sh and update this line:
  "/Users/malik/Documents/tom-protocol"
  ```
- Or add your workspace path to the SEARCH_DIRS array

### 4. **Rust Compilation Errors**

**Symptoms:**
- Cargo build fails with compilation errors
- Linker errors
- Missing dependencies

**Solution:**
```bash
# Navigate to the FFI crate directory
cd /path/to/tom-protocol-ffi

# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build --release --target aarch64-apple-tvos

# Check for specific errors in the output
```

### 5. **Missing Cargo.toml Configuration**

**Symptoms:**
- Library file not created even though build succeeds

**Solution:**
Verify your `Cargo.toml` contains:
```toml
[lib]
name = "tom_protocol_ffi"
crate-type = ["staticlib"]
```

### 6. **Xcode Build Settings Issues**

**Symptoms:**
- Script can't access environment variables
- Wrong target selected

**Solution in Xcode:**
1. Select your target in Xcode
2. Go to Build Phases
3. Find the "Run Script" phase
4. Ensure it's configured correctly:
   - Shell: `/bin/bash`
   - Script path: `${SRCROOT}/path/to/build_ffi.sh`
   - Check "Show environment variables in build log"

### 7. **Permissions Issues**

**Symptoms:**
- "Permission denied" errors
- Can't create build directory

**Solution:**
```bash
# Make script executable
chmod +x build_ffi.sh

# Check file permissions
ls -la build_ffi.sh

# If needed, fix ownership
sudo chown $(whoami) build_ffi.sh
```

## Debug Steps

### Step 1: Run the Script Manually
```bash
# Navigate to the script directory
cd /path/to/script/directory

# Run it directly
./build_ffi.sh
```

This will show you the actual error message without Xcode's wrapper.

### Step 2: Check Rust Installation
```bash
# Verify Rust is installed
which cargo
cargo --version
rustup --version

# List installed targets
rustup target list --installed | grep tvos
```

### Step 3: Test Rust Build Manually
```bash
# Navigate to the FFI crate
cd /path/to/tom-protocol-ffi

# Try building manually
cargo build --release --target aarch64-apple-tvos

# Check if the library was created
ls -la target/aarch64-apple-tvos/release/libtom_protocol_ffi.a
```

### Step 4: Check Xcode Build Logs
1. In Xcode, go to Report Navigator (⌘9)
2. Select the failed build
3. Find the "Run Script" phase
4. Look for the specific error message before "Command PhaseScriptExecution failed"

### Step 5: Verify File Paths
```bash
# From the script directory
echo "Script location: $(pwd)"

# Check if workspace exists
ls -la ../../../Cargo.toml

# Or wherever your workspace should be
ls -la /Users/malik/Documents/tom-protocol/Cargo.toml
```

## Quick Fixes

### If Rust is installed but not found by Xcode:

Add this at the top of `build_ffi.sh` (after the shebang):
```bash
# Add Rust to PATH
export PATH="$HOME/.cargo/bin:/usr/local/bin:$PATH"
```

### If the workspace path is wrong:

Edit `build_ffi.sh` and add your actual path at the beginning of SEARCH_DIRS:
```bash
SEARCH_DIRS=(
    "/your/actual/path/to/tom-protocol"  # Add this line
    "$SCRIPT_DIR/../.."
    # ... rest of the paths
)
```

### If you're building for Simulator but getting device target errors:

The script tries to detect this automatically, but you can force it:
```bash
# In build_ffi.sh, before the case statement, add:
TVOS_TARGET="aarch64-apple-tvos-sim"  # For simulator
# or
TVOS_TARGET="aarch64-apple-tvos"      # For device
```

## Still Having Issues?

1. **Check the actual error**: The generic "PhaseScriptExecution failed" message hides the real error. Look earlier in the build log for the actual problem.

2. **Run with verbose output**: Edit the script to add verbose flags:
   ```bash
   cargo build --release --target "$TVOS_TARGET" --verbose
   ```

3. **Simplify the build**: Comment out the tvOS target and try building for your host first:
   ```bash
   cargo build --release
   ```

4. **Check dependencies**: Some Rust crates don't support tvOS. Check if all your dependencies are compatible.

5. **Environment isolation**: Xcode runs scripts in a clean environment. Make sure all required tools are in standard locations or explicitly added to PATH.
