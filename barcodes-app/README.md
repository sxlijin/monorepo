# barcodes (iOS)

Simple iOS barcode manager.

## Build
From the repo root:
```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcodebuild -project barcodes.xcodeproj -scheme barcodes \
  -destination 'generic/platform=iOS Simulator' \
  clean build
```

## Architecture
- `ContentView` is the entry point
- `HomepageView` is the main view that lists all saved barcodes
- `BarcodeDetailView` is the "hold a barcode under the scanner" view
- `AddBarcodeView` and `EditBarcodeView` use `BarcodeFormView` to share logic
- add/edit UX is inspired by the iOS alarm clock UX: using a sheet, save in top right
- `BarcodeScannerView` is the barcode scanner itself
