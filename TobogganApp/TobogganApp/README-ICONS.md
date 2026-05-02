# 🎨 TobogganApp Icon Setup

This guide helps you fix the "AppIcon did not have any applicable content" error.

## 🚀 Quick Fix (Recommended)

### Option 1: Use the Python Script (Best)

```bash
# Make sure you're in the TobogganApp directory
cd TobogganApp

# Run the setup script
python3 setup-icons.py
```

**Benefits:**

- ✅ Interactive menu
- ✅ Auto-detects your icons
- ✅ Can generate all sizes from one 1024x1024 image
- ✅ Validates everything
- ✅ Creates proper Contents.json

### Option 2: Use the Shell Script

```bash
# Make the script executable
chmod +x setup-icons.sh

# Run it
./setup-icons.sh
```

### Option 3: Manual Setup in Xcode

1. Open your project in **Xcode**
2. In the left panel (Project Navigator), find and click **Assets.xcassets**
3. Click on **AppIcon**
4. In the right panel (Attributes Inspector), check **"Single Size"** (for iOS 18+)
5. Open Finder and navigate to `../logo/AppIcons/`
6. Drag your **1024×1024** icon into the single icon slot in Xcode

## 📁 What Files Do You Need?

### iOS 18+ (Easiest - Single Icon)

Just one file:

- `icon-1024.png` or `AppIcon-1024.png` (1024×1024 pixels)

### iOS 17 and Earlier (Multiple Icons)

You need these sizes:

- `AppIcon-40.png` (40×40)
- `AppIcon-60.png` (60×60)
- `AppIcon-58.png` (58×58)
- `AppIcon-87.png` (87×87)
- `AppIcon-80.png` (80×80)
- `AppIcon-120.png` (120×120)
- `AppIcon-180.png` (180×180)
- `AppIcon-1024.png` (1024×1024)

## 🛠️ Generate All Sizes from One Icon

If you only have a 1024×1024 icon and need all sizes:

### Using Python Script (Recommended)

```bash
# Install Pillow if you haven't
pip3 install Pillow

# Run the script and choose option 4
python3 setup-icons.py
```

### Using macOS sips Command

```bash
cd ../logo/AppIcons

# Generate all sizes
sips -z 180 180 icon-1024.png --out icon-180.png
sips -z 120 120 icon-1024.png --out icon-120.png
sips -z 87 87 icon-1024.png --out icon-87.png
sips -z 80 80 icon-1024.png --out icon-80.png
sips -z 60 60 icon-1024.png --out icon-60.png
sips -z 58 58 icon-1024.png --out icon-58.png
sips -z 40 40 icon-1024.png --out icon-40.png

# Then run the setup script
cd ../../TobogganApp
python3 setup-icons.py
```

### Using Online Tools

- [AppIcon.co](https://appicon.co) - Upload one, get all sizes
- [MakeAppIcon](https://makeappicon.com)
- [App Icon Generator](https://www.appicon.build)

## ✅ Icon Requirements

Your icon file(s) must:

- ✅ Be **PNG format**
- ✅ Be **square** (width = height)
- ✅ Use **RGB color space**
- ✅ Have **NO transparency** (no alpha channel) for the 1024×1024 App Store icon
- ✅ Be the **exact pixel dimensions** required

❌ Don't:

- Add rounded corners (iOS adds these automatically)
- Use transparency in the 1024×1024 version
- Include text that's too small to read at small sizes

## 🧪 Testing Your Icons

After setup:

1. **Clean Build Folder**

   - In Xcode: `Shift + Command + K`

2. **Build and Run**

   - In Xcode: `Command + R`

3. **Check Home Screen**

   - Look at your app icon in the simulator or on device

4. **Verify in Xcode**

   - Go to Assets.xcassets → AppIcon
   - All icon slots should show your icon
   - No yellow warnings should appear

## 🐛 Troubleshooting

### "No applicable content" Error

**Problem:** AppIcon is completely empty
**Solution:** Run `python3 setup-icons.py` or manually add icons in Xcode

### "Alpha channel" Error

**Problem:** Your 1024×1024 icon has transparency
**Solution:** Remove transparency:

```bash
# Using sips
sips -s format png --setProperty formatOptions normal icon-1024.png

# Or use the Python script which does this automatically
python3 setup-icons.py
```

### Icons Look Blurry

**Problem:** Icons are scaled incorrectly
**Solution:** Make sure you're providing exact pixel dimensions (not upscaled/downscaled)

### Icons Don't Update

**Problem:** Xcode cached old icons
**Solution:**

1. Clean build folder (`Shift + Command + K`)
2. Delete derived data: `rm -rf ~/Library/Developer/Xcode/DerivedData/*`
3. Build again

### Script Can't Find Icons

**Problem:** Icons are in wrong location
**Solution:**

- Verify icons are in `../logo/AppIcons/` relative to TobogganApp source directory
- Run `ls -la ../logo/AppIcons/` to see what's there

## 📝 Files Created

After running the setup scripts, you'll have:

```
TobogganApp/
├── Assets.xcassets/
│   └── AppIcon.appiconset/
│       ├── AppIcon.png (or AppIcon-XX.png files)
│       └── Contents.json
├── setup-icons.py (this script)
├── setup-icons.sh (bash alternative)
├── ICON_SETUP_GUIDE.md (detailed guide)
└── README-ICONS.md (this file)
```

## 🎯 What the Scripts Do

1. ✅ Locate your icon files in `../logo/AppIcons/`
2. ✅ Create `Assets.xcassets/AppIcon.appiconset/` if needed
3. ✅ Copy (or generate) all required icon sizes
4. ✅ Create proper `Contents.json` configuration
5. ✅ Validate everything is correct

## 💡 Tips

- **For new projects**: Use single size (iOS 18+) approach
- **For backwards compatibility**: Use multiple sizes
- **Save time**: Let the Python script generate all sizes
- **Quality**: Start with a large (1024×1024) high-quality source
- **Testing**: Test on both light and dark mode

## 🆘 Still Having Issues?

1. Check files in logo directory:

   ```bash
   ls -la ../logo/AppIcons/
   ```

2. Verify Assets.xcassets was created:

   ```bash
   ls -la Assets.xcassets/AppIcon.appiconset/
   ```

3. Check Contents.json:

   ```bash
   cat Assets.xcassets/AppIcon.appiconset/Contents.json
   ```

4. If all else fails, try the manual Xcode method (Option 3 above)

## 📚 Resources

- [Apple: App Icon Documentation](https://developer.apple.com/design/human-interface-guidelines/app-icons)
- [ICON_SETUP_GUIDE.md](./ICON_SETUP_GUIDE.md) - Detailed guide
- [Apple: Asset Catalog Format](https://developer.apple.com/library/archive/documentation/Xcode/Reference/xcode_ref-Asset_Catalog_Format/)

______________________________________________________________________

**Need more help?** Check the detailed guide: `ICON_SETUP_GUIDE.md`
