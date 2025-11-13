# Tauri Updater Setup Guide

## ✅ What's Already Done

1. **Public Key Added**: Your `tauri.conf.json` now has the public key configured
2. **Updater Store**: `src/lib/store/updater.ts` - State management for updates
3. **Updater Hook**: `src/lib/hooks/use-soft-updater.ts` - Logic for checking/downloading/installing updates
4. **UI Components**: Banner and update dialogs already implemented

## 🔐 GitHub Secrets Setup

You need to add these secrets to your GitHub repository:

### 1. Go to GitHub Settings
Navigate to: `https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions`

### 2. Add TAURI_SIGNING_PRIVATE_KEY

Click "New repository secret" and add:
- **Name**: `TAURI_SIGNING_PRIVATE_KEY`
- **Value**: Copy the ENTIRE content from `C:\Users\Mana\.tauri\osskins.key`

The private key looks like this (use YOUR actual key):
```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5ekgrWTNEOHlsc0ZCMDVJNUs0ZUZqRkF4ckpYNTQ1dXpDNnpyW
WJydWNRRUFBQkFBQUFBQUFBQUFBQUlBQUFBQURJcE1JZGFVR25vakRkM1pZUlkralB6TFRqOUxoQ09KZW9ZM1pCeXhLbkJ3TWdBdHliWDMrSmgvdmJHcnJaa3
```

### 3. Add TAURI_SIGNING_PRIVATE_KEY_PASSWORD

Click "New repository secret" and add:
- **Name**: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- **Value**: The password you entered when generating the key pair

## 📦 How to Create a Release

### Option 1: Using GitHub UI (Recommended for now)

1. Go to `https://github.com/Abdelrhmanx74/osskins/releases/new`
2. Create a new tag (e.g., `v1.5.3`)
3. Set release title and description
4. **Manually upload your build artifacts**:
   - Build locally: `pnpm tauri build`
   - Sign the artifacts (see signing section below)
   - Upload the `.msi`, `.msi.zip`, `.msi.zip.sig` files
   - Upload `updates.json` (see format below)

### Option 2: Automated with GitHub Actions (Recommended)

I've created a workflow file `.github/workflows/release.yml` that will:
- Build your app for Windows (can add other platforms)
- Sign the update artifacts automatically
- Create the `updates.json` file
- Upload everything to the release

**To trigger it**: Just create a new tag and push it:
```bash
git tag v1.5.3
git push origin v1.5.3
```

Or create a release in GitHub UI, and the workflow will run automatically.

## 🔧 Manual Signing (if not using GitHub Actions)

After building locally, you need to sign the update:

```bash
# Build the app
pnpm tauri build

# The artifacts will be in src-tauri/target/release/bundle/
# Sign the .msi.zip file
pnpm tauri signer sign "src-tauri/target/release/bundle/msi/osskins_1.5.3_x64_en-US.msi.zip" \
  -p ~/.tauri/osskins.key \
  -w "your-password"
```

This creates a `.sig` file that must be uploaded alongside the `.msi.zip`.

## 📄 updates.json Format

Create a file named `updates.json` in each release with this format:

```json
{
  "version": "1.5.3",
  "date": "2024-11-12T00:00:00.000Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSVCt1dH...",
      "url": "https://github.com/Abdelrhmanx74/osskins/releases/download/v1.5.3/osskins_1.5.3_x64_en-US.msi.zip"
    }
  },
  "body": "Release notes here..."
}
```

The signature comes from the `.sig` file created during signing.

## 🧪 Testing Updates

1. **Build and install a release version** (not dev mode):
   ```bash
   pnpm tauri build
   # Install the .msi file
   ```

2. **Create a new release** with a higher version number

3. **In the app**, click the update button in the top bar or wait for auto-check

4. **The update flow**:
   - App checks for updates
   - Downloads the new version in background
   - Shows a banner when ready
   - Click "Install" to apply and restart

## 🎯 Update Endpoints Configured

Your app is configured to check the latest release:
```
https://github.com/Abdelrhmanx74/osskins/releases/latest/download/updates.json
```

This endpoint always points to the most recent release, making it easier to manage updates without version-specific URLs.

## ⚠️ Important Notes

1. **Keep your private key safe!** Store `~/.tauri/osskins.key` securely
2. **Never commit the private key** to Git
3. **Don't lose the password** - you won't be able to sign updates without it
4. **Version numbers must increase** - Tauri only updates to higher versions
5. **Test in release mode** - Updates don't work in development builds

## 🐛 Troubleshooting

### "missing field `pubkey`" error
✅ **Fixed!** - The public key is now in `tauri.conf.json`

### Updates not detected
- Check that the `updates.json` URL is accessible
- Verify the version number is higher than current
- Check the app logs for errors

### Signature verification failed
- Make sure you're using the correct private key
- Verify the `.sig` file matches the `.msi.zip` file
- Check that the public key in `tauri.conf.json` matches your key pair

## 📚 Resources

- [Tauri Updater Documentation](https://v2.tauri.app/plugin/updater/)
- [Tauri Signer CLI](https://v2.tauri.app/reference/cli/#signer)
- [Your updater hook](src/lib/hooks/use-soft-updater.ts)
