# TomNode iOS/iPadOS — Build & Run

PROJECT         = TomNode.xcodeproj
SCHEME          = TomNode
BUNDLE_ID       = malik.karaoui.TomNode-iOS

IOS_SIM_NAME    = iPhone 16
IOS_DEST        = platform=iOS Simulator,name=$(IOS_SIM_NAME)

BUILD_DIR       = .build/xcode
LOG_DIR         = .build/logs

REPO_ROOT       = ../..
FFI_XCFW_SCRIPT = scripts/build-tom-protocol-ffi-xcframework.sh

.PHONY: help
help:
	@echo "TomNode iOS/iPadOS — Available commands:"
	@echo "  make iossim         — build iOS Simulator"
	@echo "  make iosrun         — build + install + launch iOS Simulator"
	@echo "  make ffi-xcframework — rebuild Rust XCFramework (all platforms)"
	@echo "  make devices        — list available iOS simulators"
	@echo "  make doctor         — diagnostic"
	@echo "  make clean          — clean build artifacts"

.PHONY: iossim
iossim:
	@mkdir -p $(LOG_DIR)
	@echo "▶ Build iOS Simulator..."
	xcodebuild \
		-project $(PROJECT) \
		-scheme $(SCHEME) \
		-destination '$(IOS_DEST)' \
		-configuration Debug \
		-derivedDataPath $(BUILD_DIR) \
		CODE_SIGNING_ALLOWED=NO \
		| xcpretty --color 2>/dev/null || tail -20 $(LOG_DIR)/ios_build.log
	@echo "✅ Build iOS done"

.PHONY: iosrun
iosrun: iossim
	@echo "▶ Boot iOS simulator $(IOS_SIM_NAME)..."
	@open -a Simulator
	@xcrun simctl boot "$(IOS_SIM_NAME)" 2>/dev/null || true
	@sleep 2
	@APP_PATH=$$(find $(BUILD_DIR) -name "$(SCHEME).app" | head -1); \
	if [ -z "$$APP_PATH" ]; then echo "❌ App not found"; exit 1; fi; \
	echo "▶ Install $$APP_PATH"; \
	xcrun simctl install booted "$$APP_PATH"; \
	echo "▶ Launch $(BUNDLE_ID)"; \
	xcrun simctl launch booted $(BUNDLE_ID) || true
	@echo "✅ App launched"

.PHONY: ffi-xcframework
ffi-xcframework:
	@echo "▶ Building Rust XCFramework (all platforms)..."
	cd $(REPO_ROOT) && bash $(FFI_XCFW_SCRIPT)

.PHONY: devices
devices:
	@echo "📱 Available iOS Simulators:"
	@xcrun simctl list devices available | grep -i "iphone\|ipad" | head -10 || true

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
	@echo "▶ iOS Simulators"
	@xcrun simctl list devices available | grep -i "iphone" | head -5 || true

.PHONY: clean
clean:
	@rm -rf $(BUILD_DIR) $(LOG_DIR)
	@echo "🧹 Clean done"
