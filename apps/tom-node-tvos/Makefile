# TomNode tvOS — Build & Run
# Per ios_vscode_claude_workflow_v4.md

PROJECT         = TomNode.xcodeproj
SCHEME          = TomNode
BUNDLE_ID       = malik.karaoui.TomNode

TVOS_SIM_NAME   = Apple TV 4K (3rd generation)
TVOS_DEST       = platform=tvOS Simulator,name=$(TVOS_SIM_NAME)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs
ARCHIVE_DIR     = .build/archive

# Rust FFI build (from repo root)
REPO_ROOT       = ../..
FFI_SCRIPT      = $(REPO_ROOT)/scripts/build-tom-protocol-ffi-tvos.sh

.PHONY: help
help:
	@echo "TomNode tvOS — Available commands:"
	@echo "  make tvsim      — build tvOS simulator"
	@echo "  make tvrun      — build + install + launch tvOS simulator"
	@echo "  make tvdevice   — build for physical Apple TV"
	@echo "  make tvtest     — run unit tests on simulator"
	@echo "  make ffi        — rebuild Rust FFI library (simulator)"
	@echo "  make ffi-device — rebuild Rust FFI library (device)"
	@echo "  make devices    — list available devices"
	@echo "  make doctor     — diagnostic"
	@echo "  make logs       — show build logs"
	@echo "  make clean      — clean build artifacts"
	@echo "  make archive    — archive for App Store"

.PHONY: tvsim
tvsim:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build tvOS Simulator..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(TVOS_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| tee $(LOG_DIR)/tvos_build.log \
		| xcpretty --color 2>/dev/null || tail -20 $(LOG_DIR)/tvos_build.log
	@echo "✅ Build tvOS done"

.PHONY: tvrun
tvrun: tvsim
	@echo "▶ Boot tvOS simulator $(TVOS_SIM_NAME)..."
	@open -a Simulator
	@xcrun simctl boot "$(TVOS_SIM_NAME)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME).app" -path "*tvOS*" | head -1); \
	if [ -z "$$APP_PATH" ]; then \
		APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME).app" | head -1); \
	fi; \
	if [ -z "$$APP_PATH" ]; then echo "❌ App not found"; exit 1; fi; \
	echo "▶ Install $$APP_PATH"; \
	xcrun simctl install booted "$$APP_PATH"; \
	echo "▶ Launch $(BUNDLE_ID)"; \
	xcrun simctl launch booted $(BUNDLE_ID) || true
	@echo "✅ App launched"

.PHONY: tvdevice
tvdevice:
	@if [ -z "$(DEVICE_ID)" ]; then echo "❌ DEVICE_ID required. Run: make devices"; exit 1; fi
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination 'id=$(DEVICE_ID)' \
		-configuration Debug \
		-allowProvisioningUpdates \
		| xcpretty --color 2>/dev/null || true

.PHONY: tvtest
tvtest:
	@mkdir -p $(LOG_DIR)
	xcodebuild test \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(TVOS_DEST)' \
		-derivedDataPath $(BUILD_DIR) \
		| tee $(LOG_DIR)/tests.log \
		| xcpretty --color 2>/dev/null || tail -20 $(LOG_DIR)/tests.log

.PHONY: ffi
ffi:
	@echo "▶ Building Rust FFI for tvOS simulator..."
	cd $(REPO_ROOT) && TVOS_TARGET=aarch64-apple-tvos-sim $(FFI_SCRIPT)

.PHONY: ffi-device
ffi-device:
	@echo "▶ Building Rust FFI for tvOS device..."
	cd $(REPO_ROOT) && TVOS_TARGET=aarch64-apple-tvos $(FFI_SCRIPT)

.PHONY: devices
devices:
	@echo "📱 Available devices:"
	@xcrun devicectl list devices 2>/dev/null || xcrun instruments -s devices 2>/dev/null || true
	@echo ""
	@echo "📺 tvOS Simulators:"
	@xcrun simctl list devices available | grep -i "tv" || true

.PHONY: doctor
doctor:
	@echo "▶ Xcode"
	@xcodebuild -version || true
	@echo ""
	@echo "▶ Swift"
	@swift --version || true
	@echo ""
	@echo "▶ Rust FFI library"
	@ls -lh build/libtom_protocol_ffi.a 2>/dev/null || echo "  ⚠ Not built yet. Run: make ffi"
	@echo ""
	@echo "▶ FFI Header"
	@ls -lh build/tom_protocol_ffi.h 2>/dev/null || echo "  ⚠ Header missing"
	@echo ""
	@echo "▶ tvOS Simulators"
	@xcrun simctl list devices available | grep -i "tv" | head -5 || true

.PHONY: logs
logs:
	@echo "--- tvos_build.log ---"
	@tail -50 $(LOG_DIR)/tvos_build.log 2>/dev/null || echo "(no log)"
	@echo ""
	@echo "--- tests.log ---"
	@tail -50 $(LOG_DIR)/tests.log 2>/dev/null || echo "(no log)"

.PHONY: clean
clean:
	@rm -rf $(BUILD_DIR) $(LOG_DIR) $(ARCHIVE_DIR)
	@echo "🧹 Clean done"

.PHONY: archive
archive:
	@mkdir -p $(ARCHIVE_DIR)
	xcodebuild archive \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-archivePath $(ARCHIVE_DIR)/$(SCHEME).xcarchive \
		-allowProvisioningUpdates \
		| xcpretty --color 2>/dev/null || true
