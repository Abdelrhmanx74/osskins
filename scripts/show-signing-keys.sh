#!/bin/bash
# Helper script to display your Tauri signing keys for GitHub Secrets setup

echo "=================================================="
echo "🔐 TAURI SIGNING KEYS FOR GITHUB SECRETS"
echo "=================================================="
echo ""

if [ ! -f ~/.tauri/osskins.key ]; then
    echo "❌ ERROR: Private key not found at ~/.tauri/osskins.key"
    echo "Run: pnpm tauri signer generate -w ~/.tauri/osskins.key"
    exit 1
fi

echo "✅ Keys found!"
echo ""
echo "=================================================="
echo "1️⃣  TAURI_SIGNING_PRIVATE_KEY"
echo "=================================================="
echo "Copy this ENTIRE content to GitHub Secret:"
echo ""
cat ~/.tauri/osskins.key
echo ""
echo ""

echo "=================================================="
echo "2️⃣  Public Key (already in tauri.conf.json)"
echo "=================================================="
if [ -f ~/.tauri/osskins.key.pub ]; then
    cat ~/.tauri/osskins.key.pub
else
    echo "⚠️  Public key file not found"
fi
echo ""
echo ""

echo "=================================================="
echo "📋 NEXT STEPS:"
echo "=================================================="
echo "1. Go to: https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions"
echo "2. Update or create secret: TAURI_SIGNING_PRIVATE_KEY"
echo "3. Paste the private key content shown above"
echo "4. Create secret: TAURI_SIGNING_PRIVATE_KEY_PASSWORD"
echo "5. Enter the password you used when generating the keys"
echo ""
echo "⚠️  IMPORTANT: Never commit these keys to Git!"
echo "=================================================="
