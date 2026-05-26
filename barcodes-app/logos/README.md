# Logos

Source artwork for the app icon. `1-classic.svg` is the source of truth; the PNG is
generated from it.

## Files
- `1-classic.svg` — vector source (1024×1024)
- `1-classic.png` — rendered icon used by the app

The app's actual icon is a **copy** of the PNG at
`../barcodes/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png`. After regenerating
the PNG, copy it there to update the app icon.

## Generate the PNG from the SVG

Uses [`rsvg-convert`](https://wiki.gnome.org/Projects/LibRsvg) (`brew install librsvg`):

```bash
rsvg-convert -w 1024 -h 1024 1-classic.svg -o 1-classic.png

# update the app icon with the freshly rendered PNG
cp 1-classic.png ../barcodes/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png
```

The App Store marketing icon must be 1024×1024 with **no alpha channel**. The full-bleed
white background in the SVG makes `rsvg-convert` output an opaque PNG; verify with:

```bash
sips -g pixelWidth -g pixelHeight -g hasAlpha 1-classic.png   # expect 1024, 1024, hasAlpha: no
```
