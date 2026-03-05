# Eidra

A macOS File Provider that exposes a local backing directory (`~/.eidra-storage/`) as a cloud storage location in Finder (`~/Library/CloudStorage/Eidra/`).

## Project structure

```
Eidra/                      # Container app (SwiftUI)
  EidraApp.swift            # Registers the File Provider domain on launch
  ContentView.swift         # Status UI
  Eidra.entitlements

EidraProvider/              # File Provider extension
  FileProviderExtension.swift   # NSFileProviderReplicatedExtension — routes all
                                # file operations to the backing directory
  FileProviderEnumerator.swift  # Enumerates files/folders from backing dir
  FileProviderItem.swift        # NSFileProviderItem wrapping file attributes
  EidraProvider.entitlements
  Info.plist

project.yml                 # XcodeGen spec (generates Eidra.xcodeproj)
```

## Prerequisites

- Xcode (with `xcode-select` pointed at it, not Command Line Tools)
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`)

## Building

```bash
xcodegen generate
xcodebuild -project Eidra.xcodeproj -scheme Eidra -configuration Debug build
```

## Running

Launch the app to register the File Provider domain:

```bash
open "$(xcodebuild -project Eidra.xcodeproj -scheme Eidra -configuration Debug \
  -showBuildSettings 2>/dev/null | grep '^ *BUILT_PRODUCTS_DIR =' | head -1 \
  | awk -F' = ' '{print $2}')/Eidra.app"
```

Once launched, `~/Library/CloudStorage/Eidra/` appears in Finder. Files placed there are backed by `~/.eidra-storage/`, and vice versa.

## Stopping

Remove the File Provider domain and quit:

```bash
/path/to/Eidra.app/Contents/MacOS/Eidra --remove-domain
```

This removes `~/Library/CloudStorage/Eidra/` from Finder. The extension is hosted by `fileproviderd` and stays active even after quitting the app — `--remove-domain` is the only way to fully tear it down.

## Debugging

Stream extension logs:

```bash
log stream --predicate 'subsystem == "com.roundcolors.eidra.provider"' --level info
```

## Architecture notes

- The container app only registers the domain. The extension is hosted by the system daemon `fileproviderd`, so it stays active even after you quit the app.
- Enumeration is full re-enumeration on each pass (no incremental change tracking yet).
- The app is sandboxed by default. For local development you may need to disable the sandbox (`com.apple.security.app-sandbox` → `false` in both `.entitlements` files) since `~/.eidra-storage/` is outside the sandbox container.
