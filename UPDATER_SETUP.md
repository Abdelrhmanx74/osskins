# Tauri Updater Setup Guide

# Tauri Updater (Removed)

The soft updater feature has been removed from the Osskins project. This repository no longer includes the Tauri Updater plugin or related UI and backend code.

If you need to reintroduce the updater in the future, consult the official Tauri Updater documentation:
https://tauri.app/v2/guides/release/updater/

After building locally, you need to sign the update:

```bash
# Build the app
pnpm tauri build

# The artifacts will be in src-tauri/target/release/bundle/
# Sign the .msi.zip file with your password
pnpm tauri signer sign "src-tauri/target/release/bundle/msi/osskins_1.5.3_x64_en-US.msi.zip" \
  -p ~/.tauri/osskins.key \
  --password "your-password"
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
