# TomNode tvOS — Build & Run
# Per ios_vscode_claude_workflow_v4.md

PROJECT         = TomNode.xcodeproj
SCHEME          = TomNode
SCHEME_MACOS    = TomNode-macOS
BUNDLE_ID       = malik.karaoui.TomNode
BUNDLE_ID_MACOS = malik.karaoui.TomNode-macOS

TVOS_SIM_NAME   = Apple TV 4K (3rd generation)
TVOS_DEST       = platform=tvOS Simulator,name=$(TVOS_SIM_NAME)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs
ARCHIVE_DIR     = .build/archive

# Rust FFI : consommée via le Swift Package TomProtocolKit (S2.4).
# L'artefact binaire vit dans le package ; rebuild : make ffi-xcframework.
REPO_ROOT       = ../..
PKG_XCFW        = ../../sdk/swift/TomProtocolKit/Artifacts/TomProtocolFFI.xcframework

.PHONY: help
help:
	@echo "TomNode tvOS+iOS+macOS — Available commands:"
	@echo "  make gen            — generate Xcode project from project.yml (requires xcodegen)"
	@echo "  make tvsim          — build tvOS simulator"
	@echo "  make tvrun          — build + install + launch tvOS simulator"
	@echo "  make tvdevice       — build for physical Apple TV"
	@echo "  make tvtest         — run unit tests on simulator"
	@echo "  make macbuild       — build macOS app (TomNode-macOS target)"
	@echo "  make macrun         — build + open macOS app"
	@echo "  make macdoctor      — diagnostic macOS setup"
	@echo "  make ffi-xcframework — rebuild Rust XCFramework + sync to TomProtocolKit"
	@echo "  make devices        — list available devices"
	@echo "  make doctor         — diagnostic"
	@echo "  make logs           — show build logs"
	@echo "  make clean          — clean build artifacts"
	@echo "  make archive        — archive for App Store"

.PHONY: gen
gen:
	@which xcodegen >/dev/null 2>&1 || (echo "❌ xcodegen not found. Install: brew install xcodegen"; exit 1)
	xcodegen generate
	@echo "✅ TomNode.xcodeproj generated"

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
		2>&1 | tee $(LOG_DIR)/tvos_build.log \
		| (xcpretty --color 2>/dev/null || cat > /dev/null)
	@grep -q "BUILD SUCCEEDED" $(LOG_DIR)/tvos_build.log \
		|| { echo "❌ BUILD FAILED — dernières lignes :"; tail -30 $(LOG_DIR)/tvos_build.log; exit 1; }
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
	@mkdir -p $(LOG_DIR)
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination 'id=$(DEVICE_ID)' \
		-configuration Debug \
		-allowProvisioningUpdates \
		2>&1 | tee $(LOG_DIR)/tvdevice_build.log \
		| (xcpretty --color 2>/dev/null || cat > /dev/null)
	@grep -q "BUILD SUCCEEDED" $(LOG_DIR)/tvdevice_build.log \
		|| { echo "❌ BUILD FAILED — dernières lignes :"; tail -30 $(LOG_DIR)/tvdevice_build.log; exit 1; }
	@echo "✅ Build device done"

.PHONY: tvtest
tvtest:
	@mkdir -p $(LOG_DIR)
	xcodebuild test \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(TVOS_DEST)' \
		-derivedDataPath $(BUILD_DIR) \
		2>&1 | tee $(LOG_DIR)/tests.log \
		| (xcpretty --color 2>/dev/null || cat > /dev/null)
	@grep -q "TEST SUCCEEDED" $(LOG_DIR)/tests.log \
		|| { echo "❌ TESTS FAILED — dernières lignes :"; tail -30 $(LOG_DIR)/tests.log; exit 1; }
	@echo "✅ Tests tvOS done"

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
		2>&1 | tee $(LOG_DIR)/macos_build.log \
		| (xcpretty --color 2>/dev/null || cat > /dev/null)
	@grep -q "BUILD SUCCEEDED" $(LOG_DIR)/macos_build.log \
		|| { echo "❌ BUILD FAILED — dernières lignes :"; tail -30 $(LOG_DIR)/macos_build.log; exit 1; }
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
	@echo "▶ XCFramework macOS slice (package TomProtocolKit)"
	@ls $(PKG_XCFW)/ 2>/dev/null | grep -i mac || echo "  ⚠ Slice absent — run: make ffi-xcframework"
	@echo ""
	@echo "▶ Xcode scheme"
	@xcodebuild -project $(PROJECT) -list 2>/dev/null | grep -i "$(SCHEME_MACOS)" || echo "  ⚠ Scheme '$(SCHEME_MACOS)' absent — créer la cible dans Xcode (Lot B)"
	@echo ""
	@echo "▶ Entitlements"
	@ls TomNode-macOS.entitlements 2>/dev/null && echo "  ✅ TomNode-macOS.entitlements présent" || echo "  ⚠ Manquant"

.PHONY: ffi-xcframework
ffi-xcframework:
	@echo "▶ Building Rust XCFramework (all platforms) + sync to TomProtocolKit..."
	cd $(REPO_ROOT) && bash scripts/build-tom-protocol-ffi-xcframework.sh
	cd $(REPO_ROOT) && bash scripts/sync-xcframework-to-package.sh

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
	@echo "▶ Rust XCFramework (package TomProtocolKit)"
	@ls -d $(PKG_XCFW) 2>/dev/null || echo "  ⚠ Not built yet. Run: make ffi-xcframework"
	@echo ""
	@echo "▶ FFI Header (cbindgen)"
	@ls -lh $(REPO_ROOT)/crates/tom-protocol-ffi/include/tom_protocol_ffi.h 2>/dev/null || echo "  ⚠ Header missing — run scripts/generate-ffi-header.sh"
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
	@mkdir -p $(LOG_DIR)
	xcodebuild archive \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-archivePath $(ARCHIVE_DIR)/$(SCHEME).xcarchive \
		-allowProvisioningUpdates \
		2>&1 | tee $(LOG_DIR)/archive.log \
		| (xcpretty --color 2>/dev/null || cat > /dev/null)
	@grep -q "ARCHIVE SUCCEEDED" $(LOG_DIR)/archive.log \
		|| { echo "❌ ARCHIVE FAILED — dernières lignes :"; tail -30 $(LOG_DIR)/archive.log; exit 1; }
	@echo "✅ Archive done"
