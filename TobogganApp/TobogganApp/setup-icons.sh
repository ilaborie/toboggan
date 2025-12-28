#!/bin/bash

# TobogganApp Icon Setup Script
# This script helps set up app icons from the ../logo/AppIcons/ directory

set -e

echo "🎨 TobogganApp Icon Setup Script"
echo "================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "TobogganAppApp.swift" ]; then
    echo -e "${RED}Error: This script must be run from the TobogganApp source directory${NC}"
    echo "Current directory: $(pwd)"
    exit 1
fi

# Define paths
LOGO_DIR="../logo/AppIcons"
ASSETS_DIR="Assets.xcassets"
APPICON_DIR="$ASSETS_DIR/AppIcon.appiconset"

echo "📁 Checking directories..."

# Check if logo directory exists
if [ ! -d "$LOGO_DIR" ]; then
    echo -e "${RED}Error: Logo directory not found at: $LOGO_DIR${NC}"
    echo "Please ensure your icon files are in the ../logo/AppIcons/ directory"
    exit 1
fi

echo -e "${GREEN}✓${NC} Found logo directory: $LOGO_DIR"

# List files in logo directory
echo ""
echo "📋 Files in logo directory:"
ls -lh "$LOGO_DIR"
echo ""

# Check if Assets.xcassets exists
if [ ! -d "$ASSETS_DIR" ]; then
    echo -e "${YELLOW}Warning: Assets.xcassets not found. Creating it...${NC}"
    mkdir -p "$ASSETS_DIR"
fi

# Create AppIcon.appiconset directory
echo "🔧 Setting up AppIcon.appiconset..."
mkdir -p "$APPICON_DIR"

# Ask user which approach to use
echo ""
echo "Choose setup method:"
echo "1) Single Size (iOS 18+) - Recommended, easiest"
echo "2) Multiple Sizes (iOS 17 and earlier)"
echo "3) Auto-detect and copy all sizes"
echo ""
read -p "Enter choice (1-3): " choice

case $choice in
    1)
        echo ""
        echo "🎯 Setting up single size icon..."
        
        # Look for 1024x1024 icon
        if [ -f "$LOGO_DIR/icon-1024.png" ]; then
            cp "$LOGO_DIR/icon-1024.png" "$APPICON_DIR/AppIcon.png"
            echo -e "${GREEN}✓${NC} Copied icon-1024.png"
        elif [ -f "$LOGO_DIR/AppIcon-1024.png" ]; then
            cp "$LOGO_DIR/AppIcon-1024.png" "$APPICON_DIR/AppIcon.png"
            echo -e "${GREEN}✓${NC} Copied AppIcon-1024.png"
        elif [ -f "$LOGO_DIR/icon.png" ]; then
            cp "$LOGO_DIR/icon.png" "$APPICON_DIR/AppIcon.png"
            echo -e "${GREEN}✓${NC} Copied icon.png"
        else
            echo -e "${RED}Error: Could not find 1024x1024 icon${NC}"
            echo "Looking for: icon-1024.png, AppIcon-1024.png, or icon.png"
            exit 1
        fi
        
        # Copy single-size Contents.json
        if [ -f "AppIcon-SingleSize-Contents-Template.json" ]; then
            cp "AppIcon-SingleSize-Contents-Template.json" "$APPICON_DIR/Contents.json"
            echo -e "${GREEN}✓${NC} Created Contents.json (single size)"
        else
            echo -e "${YELLOW}Warning: Template file not found, creating basic Contents.json${NC}"
            cat > "$APPICON_DIR/Contents.json" << 'EOF'
{
  "images" : [
    {
      "filename" : "AppIcon.png",
      "idiom" : "universal",
      "platform" : "ios",
      "size" : "1024x1024"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
EOF
        fi
        ;;
        
    2)
        echo ""
        echo "🎯 Setting up multiple size icons..."
        
        # Array of required sizes
        declare -a sizes=("40" "60" "58" "87" "80" "120" "180" "1024")
        
        for size in "${sizes[@]}"; do
            if [ -f "$LOGO_DIR/icon-${size}.png" ]; then
                cp "$LOGO_DIR/icon-${size}.png" "$APPICON_DIR/AppIcon-${size}.png"
                echo -e "${GREEN}✓${NC} Copied icon-${size}.png"
            elif [ -f "$LOGO_DIR/AppIcon-${size}.png" ]; then
                cp "$LOGO_DIR/AppIcon-${size}.png" "$APPICON_DIR/AppIcon-${size}.png"
                echo -e "${GREEN}✓${NC} Copied AppIcon-${size}.png"
            else
                echo -e "${YELLOW}⚠${NC} Missing icon-${size}.png (you may need to generate this)"
            fi
        done
        
        # Copy multi-size Contents.json
        if [ -f "AppIcon-Contents-Template.json" ]; then
            cp "AppIcon-Contents-Template.json" "$APPICON_DIR/Contents.json"
            echo -e "${GREEN}✓${NC} Created Contents.json (multiple sizes)"
        fi
        ;;
        
    3)
        echo ""
        echo "🎯 Auto-detecting and copying all icon files..."
        
        # Copy all PNG files from logo directory
        find "$LOGO_DIR" -name "*.png" -exec cp {} "$APPICON_DIR/" \;
        
        count=$(find "$APPICON_DIR" -name "*.png" | wc -l)
        echo -e "${GREEN}✓${NC} Copied $count icon files"
        
        # Try to determine which Contents.json to use
        if [ -f "$APPICON_DIR/icon-1024.png" ] || [ -f "$APPICON_DIR/AppIcon-1024.png" ]; then
            if [ -f "AppIcon-Contents-Template.json" ]; then
                cp "AppIcon-Contents-Template.json" "$APPICON_DIR/Contents.json"
                echo -e "${GREEN}✓${NC} Created Contents.json"
            fi
        fi
        
        echo ""
        echo -e "${YELLOW}Note: You may need to manually adjust Contents.json in Xcode${NC}"
        ;;
        
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ Icon setup complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Open your project in Xcode"
echo "2. Navigate to Assets.xcassets → AppIcon"
echo "3. Verify that icons appear correctly"
echo "4. Clean build folder (⇧⌘K)"
echo "5. Build and run (⌘R)"
echo ""
echo "If icons don't appear:"
echo "- Make sure icon files are PNG format"
echo "- Verify 1024x1024 icon has no alpha channel"
echo "- Check that file names match the Contents.json"
echo ""
