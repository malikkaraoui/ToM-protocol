# TomNode tvOS — Build & Run
# Per ios_vscode_claude_workflow_v4.md

PROJECT         = TomNode.xcodeproj
SCHEME          = TomNode
SCHEME_MACOS    = TomNode-macOS
BUNDLE_ID       = malik.karaoui.TomNode
BUNDLE_ID_MACOS = malik.karaoui.TomNode.macos

TVOS_SIM_NAME   = Apple TV 4K (3rd generation)
TVOS_DEST       = platform=tvOS Simulator,name=$(TVOS_SIM_NAME)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs
ARCHIVE_DIR     = .build/archive

# Rust FFI build (from repo root)
REPO_ROOT       = ../..
FFI_SCRIPT      = scripts/build-tom-protocol-ffi-tvos.sh

.PHONY: help
help:
	@echo "TomNode tvOS+iOS+macOS — Available commands:"
	@echo "  make tvsim          — build tvOS simulator"
	@echo "  make tvrun          — build + install + launch tvOS simulator"
	@echo "  make tvdevice       — build for physical Apple TV"
	@echo "  make tvtest         — run unit tests on simulator"
	@echo "  make macffi         — rebuild Rust XCFramework with macOS slice"
	@echo "  make macbuild       — build macOS app (requires TomNode-macOS Xcode target)"
	@echo "  make macrun         — build + open macOS app"
	@echo "  make macdoctor      — diagnostic macOS setup"
	@echo "  make ffi-xcframework — rebuild Rust XCFramework (all platforms)"
	@echo "  make ffi            — rebuild Rust FFI (tvOS simulator only)"
	@echo "  make ffi-device     — rebuild Rust FFI (tvOS device only)"
	@echo "  make devices        — list available devices"
	@echo "  make doctor         — diagnostic"
	@echo "  make logs           — show build logs"
	@echo "  make clean          — clean build artifacts"
	@echo "  make archive        — archive for App Store"

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

.PHONY: macffi
macffi:
	@echo "▶ Building Rust XCFramework with macOS slice (arm64 + x86_64)..."
	cd $(REPO_ROOT) && bash scripts/build-tom-protocol-ffi-xcframework.sh
	@echo "✅ macOS slice ready:"
	@ls build/TomProtocolFFI.xcframework/ | grep -i mac || echo "  ⚠ Slice macos-arm64 absent — check build output"

.PHONY: macbuild
macbuild:
	@echo "▶ Build macOS app ($(SCHEME_MACOS))..."
	@mkdir -p $(LOG_DIR)
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME_MACOS) \
		-destination 'platform=macOS,arch=arm64' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		| tee $(LOG_DIR)/macos_build.log \
		| xcpretty --color 2>/dev/null || tail -30 $(LOG_DIR)/macos_build.log
	@echo "✅ macOS build done"

.PHONY: macrun
macrun: macbuild
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME_MACOS).app" | head -1); \
	if [ -z "$$APP_PATH" ]; then echo "❌ App not found — run make macbuild first"; exit 1; fi; \
	echo "▶ Launch $$APP_PATH"; \
	open "$$APP_PATH"

.PHONY: macdoctor
macdoctor:
	@echo "▶ Xcode"
	@xcodebuild -version || true
	@echo ""
	@echo "▶ Rust toolchain macOS"
	@rustup target list --installed | grep darwin || echo "  ⚠ aarch64-apple-darwin not installed — run: rustup target add aarch64-apple-darwin"
	@echo ""
	@echo "▶ XCFramework macOS slice"
	@ls build/TomProtocolFFI.xcframework/ 2>/dev/null | grep -i mac || echo "  ⚠ Slice absent — run: make macffi"
	@echo ""
	@echo "▶ Xcode scheme"
	@xcodebuild -project $(PROJECT) -list 2>/dev/null | grep -i "$(SCHEME_MACOS)" || echo "  ⚠ Scheme '$(SCHEME_MACOS)' absent — créer la cible dans Xcode (Lot B)"
	@echo ""
	@echo "▶ Entitlements"
	@ls TomNode-macOS.entitlements 2>/dev/null && echo "  ✅ TomNode-macOS.entitlements présent" || echo "  ⚠ Manquant"

.PHONY: ffi-xcframework
ffi-xcframework:
	@echo "▶ Building Rust XCFramework (tvOS + iOS, all variants)..."
	cd $(REPO_ROOT) && bash scripts/build-tom-protocol-ffi-xcframework.sh

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
	@echo "▶ Rust XCFramework"
	@ls -lh build/TomProtocolFFI.xcframework 2>/dev/null || echo "  ⚠ Not built yet. Run: make ffi-xcframework"
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
