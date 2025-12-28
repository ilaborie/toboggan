#!/usr/bin/env python3
"""
TobogganApp Icon Setup Script
Helps set up app icons from the ../logo/AppIcons/ directory
Can also generate missing icon sizes from a 1024x1024 source image
"""

import os
import sys
import shutil
import json
from pathlib import Path

# Try to import PIL for image processing
try:
    from PIL import Image
    PIL_AVAILABLE = True
except ImportError:
    PIL_AVAILABLE = False

# ANSI color codes
RED = '\033[0;31m'
GREEN = '\033[0;32m'
YELLOW = '\033[1;33m'
BLUE = '\033[0;34m'
NC = '\033[0m'  # No Color

def print_colored(text, color):
    """Print colored text"""
    print(f"{color}{text}{NC}")

def print_success(text):
    print_colored(f"✓ {text}", GREEN)

def print_error(text):
    print_colored(f"✗ {text}", RED)

def print_warning(text):
    print_colored(f"⚠ {text}", YELLOW)

def print_info(text):
    print_colored(f"ℹ {text}", BLUE)

def main():
    print(f"{BLUE}🎨 TobogganApp Icon Setup Script{NC}")
    print("=" * 40)
    print()

    # Check current directory
    if not Path("TobogganAppApp.swift").exists():
        print_error("This script must be run from the TobogganApp source directory")
        print(f"Current directory: {os.getcwd()}")
        sys.exit(1)

    # Define paths
    logo_dir = Path("../logo/AppIcons")
    assets_dir = Path("Assets.xcassets")
    appicon_dir = assets_dir / "AppIcon.appiconset"

    print_info("Checking directories...")

    # Check if logo directory exists
    if not logo_dir.exists():
        print_error(f"Logo directory not found at: {logo_dir}")
        print("Please ensure your icon files are in the ../logo/AppIcons/ directory")
        sys.exit(1)

    print_success(f"Found logo directory: {logo_dir}")

    # List files in logo directory
    print()
    print("📋 Files in logo directory:")
    for file in sorted(logo_dir.glob("*")):
        if file.is_file():
            size = file.stat().st_size / 1024  # KB
            print(f"  - {file.name} ({size:.1f} KB)")
    print()

    # Create Assets.xcassets if it doesn't exist
    if not assets_dir.exists():
        print_warning("Assets.xcassets not found. Creating it...")
        assets_dir.mkdir(parents=True)

    # Create AppIcon.appiconset directory
    print_info("Setting up AppIcon.appiconset...")
    appicon_dir.mkdir(parents=True, exist_ok=True)

    # Required icon sizes
    ICON_SIZES = {
        40: "20@2x (Notification)",
        60: "20@3x (Notification)",
        58: "29@2x (Settings)",
        87: "29@3x (Settings)",
        80: "40@2x (Spotlight)",
        120: "40@3x (Spotlight) / 60@2x (App)",
        180: "60@3x (App)",
        1024: "App Store"
    }

    # Ask user which approach to use
    print()
    print("Choose setup method:")
    print("1) Single Size (iOS 18+) - Recommended, easiest")
    print("2) Multiple Sizes (iOS 17 and earlier)")
    print("3) Auto-detect and copy all sizes")
    if PIL_AVAILABLE:
        print("4) Generate all sizes from 1024x1024 icon (requires Pillow)")
    print()

    choice = input("Enter choice (1-4): ").strip()

    if choice == "1":
        setup_single_size(logo_dir, appicon_dir)
    elif choice == "2":
        setup_multiple_sizes(logo_dir, appicon_dir, ICON_SIZES)
    elif choice == "3":
        setup_auto_detect(logo_dir, appicon_dir)
    elif choice == "4" and PIL_AVAILABLE:
        generate_all_sizes(logo_dir, appicon_dir, ICON_SIZES)
    else:
        print_error("Invalid choice")
        sys.exit(1)

    print()
    print_success("Icon setup complete!")
    print()
    print("Next steps:")
    print("1. Open your project in Xcode")
    print("2. Navigate to Assets.xcassets → AppIcon")
    print("3. Verify that icons appear correctly")
    print("4. Clean build folder (⇧⌘K)")
    print("5. Build and run (⌘R)")
    print()

def setup_single_size(logo_dir, appicon_dir):
    """Setup single 1024x1024 icon for iOS 18+"""
    print()
    print_info("Setting up single size icon...")

    # Look for 1024x1024 icon
    source_file = None
    for filename in ["icon-1024.png", "AppIcon-1024.png", "icon.png", "AppIcon.png"]:
        filepath = logo_dir / filename
        if filepath.exists():
            source_file = filepath
            break

    if not source_file:
        print_error("Could not find 1024x1024 icon")
        print("Looking for: icon-1024.png, AppIcon-1024.png, icon.png, or AppIcon.png")
        sys.exit(1)

    # Copy icon
    dest_file = appicon_dir / "AppIcon.png"
    shutil.copy2(source_file, dest_file)
    print_success(f"Copied {source_file.name}")

    # Create Contents.json
    contents = {
        "images": [
            {
                "filename": "AppIcon.png",
                "idiom": "universal",
                "platform": "ios",
                "size": "1024x1024"
            }
        ],
        "info": {
            "author": "xcode",
            "version": 1
        }
    }

    contents_file = appicon_dir / "Contents.json"
    with open(contents_file, 'w') as f:
        json.dump(contents, f, indent=2)

    print_success("Created Contents.json (single size)")

def setup_multiple_sizes(logo_dir, appicon_dir, icon_sizes):
    """Setup multiple icon sizes"""
    print()
    print_info("Setting up multiple size icons...")

    copied = []
    missing = []

    for size in icon_sizes.keys():
        source_file = None
        for filename in [f"icon-{size}.png", f"AppIcon-{size}.png"]:
            filepath = logo_dir / filename
            if filepath.exists():
                source_file = filepath
                break

        if source_file:
            dest_file = appicon_dir / f"AppIcon-{size}.png"
            shutil.copy2(source_file, dest_file)
            print_success(f"Copied {source_file.name} ({icon_sizes[size]})")
            copied.append(size)
        else:
            print_warning(f"Missing icon-{size}.png ({icon_sizes[size]})")
            missing.append(size)

    # Create Contents.json
    contents = create_multi_size_contents()
    contents_file = appicon_dir / "Contents.json"
    with open(contents_file, 'w') as f:
        json.dump(contents, f, indent=2)

    print_success("Created Contents.json (multiple sizes)")
    print()
    print(f"Copied: {len(copied)} icons")
    if missing:
        print(f"Missing: {len(missing)} icons")

def setup_auto_detect(logo_dir, appicon_dir):
    """Auto-detect and copy all icon files"""
    print()
    print_info("Auto-detecting and copying all icon files...")

    # Copy all PNG files
    count = 0
    for png_file in logo_dir.glob("*.png"):
        shutil.copy2(png_file, appicon_dir / png_file.name)
        count += 1

    print_success(f"Copied {count} icon files")

    # Create appropriate Contents.json
    if (appicon_dir / "icon-1024.png").exists() or (appicon_dir / "AppIcon-1024.png").exists():
        contents = create_multi_size_contents()
        contents_file = appicon_dir / "Contents.json"
        with open(contents_file, 'w') as f:
            json.dump(contents, f, indent=2)
        print_success("Created Contents.json")
    else:
        print_warning("You may need to manually adjust Contents.json in Xcode")

def generate_all_sizes(logo_dir, appicon_dir, icon_sizes):
    """Generate all icon sizes from a 1024x1024 source"""
    print()
    print_info("Generating all icon sizes from source image...")

    if not PIL_AVAILABLE:
        print_error("Pillow library not installed. Install with: pip3 install Pillow")
        sys.exit(1)

    # Find source image
    source_file = None
    for filename in ["icon-1024.png", "AppIcon-1024.png", "icon.png", "AppIcon.png"]:
        filepath = logo_dir / filename
        if filepath.exists():
            source_file = filepath
            break

    if not source_file:
        print_error("Could not find source icon (looking for 1024x1024 image)")
        sys.exit(1)

    print_info(f"Using source: {source_file.name}")

    # Open source image
    try:
        img = Image.open(source_file)
        
        # Check if image is 1024x1024
        if img.size != (1024, 1024):
            print_warning(f"Source image is {img.width}x{img.height}, expected 1024x1024")
            print_info("Resizing to 1024x1024...")
            img = img.resize((1024, 1024), Image.Resampling.LANCZOS)

        # Convert to RGB (remove alpha channel for App Store icon)
        if img.mode == 'RGBA':
            print_info("Converting RGBA to RGB (removing transparency)")
            rgb_img = Image.new('RGB', img.size, (255, 255, 255))
            rgb_img.paste(img, mask=img.split()[3])  # Use alpha channel as mask
            img = rgb_img

        # Generate all sizes
        for size in icon_sizes.keys():
            resized = img.resize((size, size), Image.Resampling.LANCZOS)
            output_path = appicon_dir / f"AppIcon-{size}.png"
            resized.save(output_path, 'PNG', optimize=True)
            print_success(f"Generated AppIcon-{size}.png ({icon_sizes[size]})")

        # Create Contents.json
        contents = create_multi_size_contents()
        contents_file = appicon_dir / "Contents.json"
        with open(contents_file, 'w') as f:
            json.dump(contents, f, indent=2)

        print_success("Created Contents.json")
        print()
        print_success("All icon sizes generated successfully!")

    except Exception as e:
        print_error(f"Error processing image: {e}")
        sys.exit(1)

def create_multi_size_contents():
    """Create Contents.json for multiple sizes"""
    return {
        "images": [
            {
                "filename": "AppIcon-40.png",
                "idiom": "iphone",
                "scale": "2x",
                "size": "20x20"
            },
            {
                "filename": "AppIcon-60.png",
                "idiom": "iphone",
                "scale": "3x",
                "size": "20x20"
            },
            {
                "filename": "AppIcon-58.png",
                "idiom": "iphone",
                "scale": "2x",
                "size": "29x29"
            },
            {
                "filename": "AppIcon-87.png",
                "idiom": "iphone",
                "scale": "3x",
                "size": "29x29"
            },
            {
                "filename": "AppIcon-80.png",
                "idiom": "iphone",
                "scale": "2x",
                "size": "40x40"
            },
            {
                "filename": "AppIcon-120.png",
                "idiom": "iphone",
                "scale": "3x",
                "size": "40x40"
            },
            {
                "filename": "AppIcon-120.png",
                "idiom": "iphone",
                "scale": "2x",
                "size": "60x60"
            },
            {
                "filename": "AppIcon-180.png",
                "idiom": "iphone",
                "scale": "3x",
                "size": "60x60"
            },
            {
                "filename": "AppIcon-1024.png",
                "idiom": "ios-marketing",
                "scale": "1x",
                "size": "1024x1024"
            }
        ],
        "info": {
            "author": "xcode",
            "version": 1
        }
    }

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
        print_warning("Operation cancelled by user")
        sys.exit(0)
    except Exception as e:
        print()
        print_error(f"Unexpected error: {e}")
        sys.exit(1)
