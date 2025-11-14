# Tauri Updater (Removed)

## ✅ What's Been Done

This document has been archived; the Tauri soft updater feature has been removed from the repository.

### ✨ Features Implemented

1. **Automatic Update Checking** - Checks on app startup
2. **User-Friendly Update Banner** - Shows update availability with version info
3. **Download Progress Tracking** - Real-time progress bar during downloads
4. **One-Click Installation** - Install and restart with a single click
5. **Error Handling & Retry** - Graceful error handling with retry options
6. **Comprehensive Logging** - Detailed console logs for debugging
7. **Signed Updates** - Cryptographically signed for security
8. **GitHub Actions Automation** - Automated build and release process

---

## 🔐 Required: GitHub Secrets Setup

You **MUST** set up these secrets for the updater to work. Without them, builds will fail.

### Step 1: Generate Signing Keys (if not already done)

If you haven't generated your signing keys yet, run:

```bash
pnpm tauri signer generate -- -w ~/.tauri/osskins.key
```

**On Windows:**
```bash
pnpm tauri signer generate -- -w C:\Users\YourName\.tauri\osskins.key
```

This will:
- Prompt you for a password (required - remember this!)
- Create a **private key** at the specified location
- Create a **public key** with `.pub` extension

**⚠️ The password is REQUIRED** - Tauri always encrypts the private key with a password for security.

The public key is already configured in your `tauri.conf.json` ✅

### Step 2: Add GitHub Secrets

Go to: `https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions`

Click "New repository secret" and add **TWO** secrets:

#### Secret 1: TAURI_SIGNING_PRIVATE_KEY

- **Name**: `TAURI_SIGNING_PRIVATE_KEY`
- **Value**: Copy the ENTIRE content from your private key file

**On Windows**: Open `C:\Users\YourName\.tauri\osskins.key` in Notepad
**On Mac/Linux**: Run `cat ~/.tauri/osskins.key` and copy the output

The key looks like this (use YOUR actual key):
```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5...
```

#### Secret 2: TAURI_SIGNING_PRIVATE_KEY_PASSWORD

- **Name**: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- **Value**: The password you entered when generating the keys
- **This secret is REQUIRED** - Tauri always uses password-protected keys

---

## 📦 How to Create a Release

### Option 1: Push a Tag (Recommended)

```bash
# Commit your changes first
git add .
git commit -m "Your changes"
git push

# Create and push a version tag
git tag v1.5.3
git push origin v1.5.3
```

### Option 2: Create via GitHub UI

1. Go to `https://github.com/Abdelrhmanx74/osskins/releases/new`
2. Click "Choose a tag" and type `v1.5.3` (or your version)
3. Click "Create new tag: v1.5.3 on publish"
4. Add a title and description
5. Click "Publish release"

### What Happens Automatically

Once you push a tag, GitHub Actions will:

1. ✅ Build your app for Windows
2. ✅ Sign the update artifacts with your private key
3. ✅ Generate `updates.json` with signatures
4. ✅ Create a GitHub release
5. ✅ Upload all files:
   - `osskins_X.X.X_x64_en-US.msi` (installer)
   - `osskins_X.X.X_x64_en-US.msi.zip` (updater bundle)
   - `osskins_X.X.X_x64_en-US.msi.zip.sig` (signature)
   - `updates.json` (update manifest)

---

## 🎯 How Updates Work

### For End Users

1. **User opens the app** → Automatic check for updates
2. **Update available** → Banner appears at the top
3. **User clicks "Download"** → Progress bar shows download
4. **Download complete** → "Install" button appears
5. **User clicks "Install"** → App installs and restarts automatically

### Update Endpoint

Your app checks this URL for updates:
```
https://github.com/Abdelrhmanx74/osskins/releases/latest/download/updates.json
```

This always points to your **latest release**, so you don't need version-specific URLs!

---

## 🧪 Testing Updates

### Test Scenario 1: First Installation

1. Build and install v1.5.2:
   ```bash
   pnpm tauri build
   # Install the generated .msi file
   ```

2. Create a new release v1.5.3 (see above)

3. Open the installed app → Should show update banner

### Test Scenario 2: Manual Check

1. Open the app
2. Click the update button in the top bar
3. Should check for updates and show result

### Common Test Issues

**"No update available" when you expect one:**
- Check that the new version is higher (1.5.3 > 1.5.2)
- Check that `updates.json` exists in the release
- Check browser console for error messages

**"Signature verification failed":**
- Make sure you're using the same key pair
- Verify the public key in `tauri.conf.json` matches your `.key.pub`

---

## 🐛 Troubleshooting

### Check Logs

The app now has comprehensive logging. Open DevTools (Ctrl+Shift+I) and check the console:

```
[Updater] Checking for updates. Current version: 1.5.2
[Updater] Update available: 1.5.3 (current: 1.5.2)
[Updater] Starting download...
[Updater] Download started. Total size: 45670912 bytes
[Updater] Download finished
```

### Common Issues

**Build fails with "TAURI_SIGNING_PRIVATE_KEY not set":**
- You haven't added the GitHub secret
- Go to Settings → Secrets → Actions and add it

**Updates not detected:**
- Check the release has `updates.json`
- Verify the JSON is valid (use a JSON validator)
- Make sure the version number is higher

**Signature errors:**
- Regenerate your keys and update both the GitHub secret and `tauri.conf.json`

---

## ⚠️ Important Security Notes

### DO:
- ✅ Keep your private key safe and secure
- ✅ Back up your private key securely
- ✅ Use a strong password
- ✅ Only store the key in GitHub Secrets

### DON'T:
- ❌ Commit the private key to Git
- ❌ Share the private key with anyone
- ❌ Lose the password (you can't recover it)
- ❌ Reuse keys across different apps

### Key Management

**If you lose your private key:**
- You CANNOT sign new updates for users with the old public key
- Users will need to manually download and install a new version
- Generate new keys and update your app with the new public key

---

## 📊 Monitoring Updates

### Check Update Success

After releasing, you can monitor:

1. **GitHub Release Stats** - See download counts
2. **App Logs** - Users will see update checks in console
3. **Error Reports** - Check GitHub Issues for update problems

### Release Checklist

Before each release:
- [ ] Increment version in `package.json` and `tauri.conf.json`
- [ ] Update `CHANGELOG.md` or release notes
- [ ] Test the app locally
- [ ] Create and push the version tag
- [ ] Wait for GitHub Actions to complete
- [ ] Verify the release has all files
- [ ] Test the update on an older version

---

## 🔄 Workflow Diagram

```
Developer                    GitHub Actions              End User App
    |                              |                           |
    |-- git tag v1.5.3 ---------> |                           |
    |-- git push origin v1.5.3 -> |                           |
    |                              |                           |
    |                        [Build & Sign]                    |
    |                              |                           |
    |                        [Create Release]                  |
    |                              |                           |
    |                        [Upload Artifacts]                |
    |                              |                           |
    |                              |                           |
    |                              | <--Check for updates---- |
    |                              |                           |
    |                              | ---updates.json--------> |
    |                              |                           |
    |                              | <--Download bundle------ |
    |                              |                           |
    |                              | ---signed .msi.zip-----> |
    |                              |                           |
    |                              |                      [Install]
    |                              |                           |
    |                              |                      [Restart]
```

---

## 📝 Summary

Your updater is **100% configured and ready to use!** Here's what you need to do:

1. ✅ **Add GitHub Secrets** (MUST DO)
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

2. ✅ **Create a Release**
   - Push a version tag: `git tag v1.5.3 && git push origin v1.5.3`
   - OR use GitHub UI

3. ✅ **That's it!**
   - Users will automatically get updates
   - Everything is automated

---

## 🎉 You're Done!

The updater is production-ready with:
- ✅ Secure signed updates
- ✅ Automatic checking
- ✅ User-friendly UI
- ✅ Progress tracking
- ✅ Error handling
- ✅ Comprehensive logging
- ✅ Automated releases

Just add those GitHub secrets and you're good to go! 🚀
