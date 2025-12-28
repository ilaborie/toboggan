# App Icon Setup Guide for TobogganApp

## Current Issue
The AppIcon asset is empty or missing required icon files.

## Location of Icon Files
Your icon files should be in: `../logo/AppIcons/`

## Quick Fix Steps

### Option 1: Using Xcode (Easiest)

1. Open your project in Xcode
2. In the Project Navigator (left panel), find and click on `Assets.xcassets`
3. Click on `AppIcon` in the asset list
4. You should see a grid of empty icon slots
5. Navigate to your `../logo/AppIcons/` folder in Finder
6. Drag and drop icon files into the appropriate slots based on their size

### Option 2: Command Line Setup

If you have properly sized icons (like icon-1024.png, icon-180.png, etc.), you can copy them directly:

```bash
# Navigate to your project directory
cd /path/to/TobogganApp

# Create the AppIcon.appiconset directory if needed
mkdir -p Assets.xcassets/AppIcon.appiconset

# Copy your icons (adjust paths as needed)
cp ../logo/AppIcons/icon-1024.png Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png
cp ../logo/AppIcons/icon-180.png Assets.xcassets/AppIcon.appiconset/AppIcon-180.png
# ... repeat for other sizes
```

### Option 3: Use Single 1024x1024 Icon (iOS 18+)

iOS 18 and later can automatically generate all required sizes from a single 1024×1024 icon:

1. In Xcode, select your `AppIcon` asset
2. In the Attributes Inspector (right panel), check "Single Size"
3. Drag your 1024×1024 icon into the single slot

## Required Icon Sizes

### For iOS 18+ (Recommended - Single Size)
- **1024×1024** - Universal size (Xcode generates all other sizes)

### For iOS 17 and earlier (Multiple Sizes)

#### iPhone
- 20×20 @ 2x (40×40 pixels)
- 20×20 @ 3x (60×60 pixels)
- 29×29 @ 2x (58×58 pixels)
- 29×29 @ 3x (87×87 pixels)
- 40×40 @ 2x (80×80 pixels)
- 40×40 @ 3x (120×120 pixels)
- 60×60 @ 2x (120×120 pixels)
- 60×60 @ 3x (180×180 pixels)

#### App Store
- 1024×1024 (1 image, no alpha channel)

## Icon File Naming Convention

If your icons in `../logo/AppIcons/` follow a naming pattern, here's how they map:

| File Name | Purpose | Size |
|-----------|---------|------|
| icon-1024.png or AppIcon-1024.png | App Store | 1024×1024 |
| icon-180.png or AppIcon-180.png | iPhone App @3x | 180×180 |
| icon-120.png or AppIcon-120.png | iPhone App @2x / Spotlight @3x | 120×120 |
| icon-87.png or AppIcon-87.png | Settings @3x | 87×87 |
| icon-80.png or AppIcon-80.png | Spotlight @2x | 80×80 |
| icon-60.png or AppIcon-60.png | Notification @3x | 60×60 |
| icon-58.png or AppIcon-58.png | Settings @2x | 58×58 |
| icon-40.png or AppIcon-40.png | Notification @2x | 40×40 |

## Icon Design Requirements

✅ **Must have:**
- Square shape (width = height)
- PNG format
- RGB color space
- No transparency (no alpha channel)

❌ **Avoid:**
- Rounded corners (iOS adds these automatically)
- Text that's too small to read
- Alpha channel in the 1024×1024 App Store icon

## Troubleshooting

### If you see "no applicable content"
This means the AppIcon asset is completely empty. You need to add at least one icon file.

### If you get "alpha channel" errors
Save your 1024×1024 icon without transparency. In most image editors:
- Flatten all layers
- Remove alpha channel
- Save as PNG with RGB color mode only

### If icons appear blurry
Make sure you're providing the exact pixel dimensions required (not upscaled or downscaled).

## Quick Test

After adding icons:
1. Clean build folder: Shift+Cmd+K in Xcode
2. Build and run: Cmd+R
3. Check the simulator/device home screen for your app icon

## Automated Icon Generation

If you only have one large icon, you can use tools to generate all sizes:

### Using sips (built-in macOS tool)
```bash
# From your 1024×1024 icon, generate other sizes
cd ../logo/AppIcons
sips -z 180 180 icon-1024.png --out icon-180.png
sips -z 120 120 icon-1024.png --out icon-120.png
sips -z 87 87 icon-1024.png --out icon-87.png
sips -z 80 80 icon-1024.png --out icon-80.png
sips -z 60 60 icon-1024.png --out icon-60.png
sips -z 58 58 icon-1024.png --out icon-58.png
sips -z 40 40 icon-1024.png --out icon-40.png
```

### Online Tools
- [AppIcon.co](https://appicon.co) - Upload one image, download all sizes
- [MakeAppIcon](https://makeappicon.com) - Similar service
- [App Icon Generator](https://www.appicon.build) - Another option

## Need Help?

If you're still having issues, please provide:
1. List of files in `../logo/AppIcons/` directory
2. Output of: `ls -la ../logo/AppIcons/`
3. Any specific error messages from Xcode
