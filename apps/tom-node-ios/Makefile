# TomNode iOS/iPadOS — Build & Run
PROJECT         = TomNode.xcodeproj
SCHEME          = TomNode
BUNDLE_ID       = malik.karaoui.TomNode-iOS

IOS_SIM_NAME    = iPhone 16
IOS_SIM_IPAD    = iPad Pro 13-inch (M4)
IOS_DEST        = platform=iOS Simulator,name=$(IOS_SIM_NAME)
IPAD_DEST       = platform=iOS Simulator,name=$(IOS_SIM_IPAD)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs
ARCHIVE_DIR     = .build/archive

REPO_ROOT       = ../..
FFI_XCFW        = ../tom-node-tvos/build/TomProtocolFFI.xcframework

.PHONY: help
help:
	@echo "TomNode iOS/iPadOS — Available commands:"
	@echo "  make gen            — generate Xcode project from project.yml (requires xcodegen)"
	@echo "  make iossim         — build for iPhone simulator"
	@echo "  make ipad           — build for iPad simulator"
	@echo "  make iosrun         — build + launch iPhone simulator"
	@echo "  make ipad-run       — build + launch iPad simulator"
	@echo "  make device         — build for physical device (DEVICE_ID=<id> required)"
	@echo "  make ffi-xcframework — rebuild Rust XCFramework (all platforms)"
	@echo "  make devices        — list available simulators + physical devices"
	@echo "  make doctor         — diagnostic"
	@echo "  make clean          — clean build artifacts"

.PHONY: gen
gen:
	@which xcodegen >/dev/null 2>&1 || (echo "❌ xcodegen not found. Install: brew install xcodegen"; exit 1)
	xcodegen generate
	@echo "✅ TomNode.xcodeproj generated"

.PHONY: iossim
iossim:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build iOS Simulator (iPhone)..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(IOS_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| tee $(LOG_DIR)/ios_build.log \
		| xcpretty --color 2>/dev/null || tail -20 $(LOG_DIR)/ios_build.log
	@echo "✅ Build iPhone done"

.PHONY: ipad
ipad:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build iOS Simulator (iPad)..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(IPAD_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| tee $(LOG_DIR)/ipad_build.log \
		| xcpretty --color 2>/dev/null || tail -20 $(LOG_DIR)/ipad_build.log
	@echo "✅ Build iPad done"

.PHONY: iosrun
iosrun: iossim
	@echo "▶ Boot iOS simulator $(IOS_SIM_NAME)..."
	@open -a Simulator
	@xcrun simctl boot "$(IOS_SIM_NAME)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME).app" | head -1); \
	[ -z "$$APP_PATH" ] && echo "❌ App not found" && exit 1; \
	xcrun simctl install booted "$$APP_PATH"; \
	xcrun simctl launch booted $(BUNDLE_ID) || true
	@echo "✅ Launched on iPhone simulator"

.PHONY: ipad-run
ipad-run: ipad
	@echo "▶ Boot iPad simulator $(IOS_SIM_IPAD)..."
	@open -a Simulator
	@xcrun simctl boot "$(IOS_SIM_IPAD)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME).app" | head -1); \
	[ -z "$$APP_PATH" ] && echo "❌ App not found" && exit 1; \
	xcrun simctl install booted "$$APP_PATH"; \
	xcrun simctl launch booted $(BUNDLE_ID) || true
	@echo "✅ Launched on iPad simulator"

.PHONY: device
device:
	@[ -n "$(DEVICE_ID)" ] || (echo "❌ DEVICE_ID required. Run: make devices"; exit 1)
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination 'id=$(DEVICE_ID)' \
		-configuration Debug \
		-allowProvisioningUpdates \
		| xcpretty --color 2>/dev/null || true

.PHONY: ffi-xcframework
ffi-xcframework:
	@echo "▶ Building Rust XCFramework (all platforms)..."
	cd $(REPO_ROOT) && bash scripts/build-tom-protocol-ffi-xcframework.sh

.PHONY: devices
devices:
	@echo "📱 iOS Simulators:"
	@xcrun simctl list devices available | grep -E "iPhone|iPad" | head -20 || true
	@echo ""
	@echo "📱 Physical devices:"
	@xcrun devicectl list devices 2>/dev/null | head -20 || xcrun instruments -s devices 2>/dev/null | head -20 || true

.PHONY: doctor
doctor:
	@echo "▶ Xcode"
	@xcodebuild -version || true
	@echo ""
	@echo "▶ xcodegen"
	@xcodegen --version 2>/dev/null || echo "  ⚠ Not installed. Run: brew install xcodegen"
	@echo ""
	@echo "▶ Xcode project"
	@ls -d $(PROJECT) 2>/dev/null && echo "  ✅ project exists" || echo "  ⚠ Not generated. Run: make gen"
	@echo ""
	@echo "▶ Rust XCFramework"
	@ls -lh $(FFI_XCFW) 2>/dev/null && echo "  ✅ XCFramework OK" || echo "  ⚠ Not built. Run: make ffi-xcframework"
	@echo ""
	@echo "▶ FFI Header"
	@ls -lh ../tom-node-tvos/build/tom_protocol_ffi.h 2>/dev/null && echo "  ✅ header OK" || echo "  ⚠ Header missing"
	@echo ""
	@echo "▶ iOS Simulators"
	@xcrun simctl list devices available | grep -E "iPhone 1[5-9]|iPad Pro" | head -5 || true

.PHONY: clean
clean:
	@rm -rf $(BUILD_DIR) $(LOG_DIR) $(ARCHIVE_DIR)
	@echo "🧹 Clean done"
