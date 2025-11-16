# Tauri v2 Soft Updater - Implementation Summary

## Overview

This document describes the complete implementation of the Tauri v2 soft updater for the Osskins application. The updater allows users to automatically check for and install app updates from GitHub releases.

## What's Been Implemented

### 1. Backend Configuration

#### Cargo.toml
- Added `tauri-plugin-updater = "2.0.0"` dependency
- Added `tauri-plugin-process = "2.0.0"` for app relaunch functionality

#### main.rs
- Initialized the updater plugin: `.plugin(tauri_plugin_updater::Builder::new().build())`
- Initialized the process plugin: `.plugin(tauri_plugin_process::init())`

#### tauri.conf.json
- Enabled updater artifacts creation: `"createUpdaterArtifacts": true`
- Configured updater with public key for signature verification
- Set update endpoint to GitHub releases: `https://github.com/Abdelrhmanx74/osskins/releases/latest/download/updates.json`
- Configured Windows install mode as "passive" for silent updates

### 2. Frontend Implementation

#### Dependencies (package.json)
- Added `@tauri-apps/plugin-updater@^2.4.1`
- Added `@tauri-apps/plugin-process@^2.0.0`

#### State Management (src/lib/store/updater.ts)
Implemented a Zustand store with the following state:
- `status`: Current update status (idle, checking, available, downloading, downloaded, etc.)
- `currentVersion`: App's current version
- `availableVersion`: Available update version
- `releaseNotes`: Changelog/release notes
- `updateHandle`: Reference to the update object
- `progress`: Download progress percentage
- `downloadedBytes` / `totalBytes`: Download progress tracking
- `error`: Error message if any
- `bannerDismissed`: Whether user dismissed the update banner

#### Update Hook (src/lib/hooks/use-soft-updater.ts)
Provides the following functionality:
- `checkForUpdates()`: Check for available updates from GitHub
- `downloadUpdate()`: Download the update with progress tracking
- `installUpdate()`: Install and relaunch the app
- `dismissBanner()`: Dismiss update notification
- `showBanner()`: Show update notification
- Auto-check on mount (configurable)

#### UI Components

**AppUpdateDialog** (src/components/AppUpdateDialog.tsx)
A comprehensive dialog that shows:
- Update checking status with spinner
- Update availability with version info and release notes
- Download progress with progress bar
- Download completion with install button
- Error states with retry option
- Up-to-date confirmation

**ReleaseNotesDialog** (src/components/ReleaseNotesDialog.tsx)
Shows release history:
- Fetches releases from GitHub API
- Displays version tags, release dates, and notes
- Highlights current version
- Shows pre-release badges
- Links to GitHub release pages

#### Integration Points

**TopBar Menu**
- Added "Check for App Updates" menu item with badge when update available
- Added "Release Notes" menu item
- Shows update indicator badge on menu button when update available
- Auto-checks for updates on app startup

**SettingsDialog**
- Added "Application Updates" section showing:
  - Current version display
  - Update availability badge
  - "Check for Updates" button
  - "Release Notes" button
  - Information about automatic checks
- Separated data updates from app updates for clarity

### 3. Release Workflow

The existing GitHub Actions workflow (`.github/workflows/release.yml`) already:
- Builds the app on tag push (v*.*.* pattern)
- Signs update artifacts with private key
- Generates `updates.json` manifest
- Creates GitHub releases with all necessary files:
  - `.msi` installer
  - `.msi.zip` update bundle
  - `.msi.zip.sig` signature file
  - `updates.json` manifest

## How It Works

### Update Flow

1. **App Startup**
   - Updater hook auto-checks for updates (silent mode)
   - If update available, badge appears on TopBar menu

2. **User Interaction**
   - User clicks "Check for App Updates" in menu or Settings
   - Dialog shows update information and release notes
   - User clicks "Download Update"

3. **Download**
   - Update downloads with progress tracking
   - Progress bar shows download percentage
   - On completion, "Install & Restart" button appears

4. **Installation**
   - User clicks "Install & Restart"
   - App installs update and relaunches automatically
   - New version starts after relaunch

### Update Checking Logic

```typescript
// Auto-check on mount (silent)
useSoftUpdater({ autoCheck: true });

// Manual check (shows toast notifications)
await checkForUpdates({ silent: false });
```

### Download Progress Tracking

The updater tracks three download events:
- `Started`: Initializes progress with content length
- `Progress`: Updates cumulative download progress
- `Finished`: Marks download as complete

## Security

### Signature Verification
- All updates are cryptographically signed
- Public key is embedded in `tauri.conf.json`
- Tauri automatically verifies signatures before installation
- Invalid signatures are rejected

### Private Key Management
The private key must be stored in GitHub Secrets:
- `TAURI_SIGNING_PRIVATE_KEY`: The private key content
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Key password

## Testing

### Manual Testing Steps

1. **Build version 1.5.2**
   ```bash
   pnpm tauri build
   ```

2. **Install the app**

3. **Create a new release v1.5.3**
   ```bash
   git tag v1.5.3
   git push origin v1.5.3
   ```

4. **Wait for GitHub Actions to complete**

5. **Open the installed app**
   - Should show update notification
   - Menu badge should appear
   - Check update dialog should show v1.5.3 available

6. **Download and install update**
   - Click "Download Update"
   - Watch progress bar
   - Click "Install & Restart"
   - App should relaunch with v1.5.3

### Development Testing

For development without building:
- The updater only works in Tauri production builds
- Use `isTauriEnvironment()` check to prevent errors in dev mode
- Mock the updater store in tests if needed

## Features

### ✅ Implemented
- Automatic update checking on startup
- Manual update checking via UI
- Download progress tracking
- One-click installation
- Release notes viewing
- Version comparison
- Error handling and retry
- Signed updates
- Badge notifications
- GitHub API integration for release history

### 📋 Future Enhancements (Optional)
- Update changelog formatting (markdown rendering)
- Update size display before download
- Update scheduling (install on next restart)
- Auto-download in background (with user consent)
- Update history/rollback
- Custom update intervals

## Troubleshooting

### Common Issues

**"No update available" when expecting one:**
- Verify the version in `tauri.conf.json` and `package.json` match
- Ensure the new version is higher than current
- Check that `updates.json` exists in the GitHub release
- Verify the endpoint URL is correct

**"Signature verification failed":**
- Public key in `tauri.conf.json` must match the private key used to sign
- Regenerate keys if they don't match
- Update GitHub secrets with new private key

**Update downloads but won't install:**
- Check Windows install mode in `tauri.conf.json`
- Verify app has permissions to install
- Check for error messages in console

**TypeScript errors:**
- Ensure all plugin packages are installed
- Run `pnpm install` after adding dependencies
- Check that types are properly imported

## Files Modified/Created

### Created
- `src/components/AppUpdateDialog.tsx`
- `src/components/ReleaseNotesDialog.tsx`

### Modified
- `package.json` - Added updater dependencies
- `pnpm-lock.yaml` - Updated lockfile
- `src-tauri/Cargo.toml` - Added Rust dependencies
- `src-tauri/tauri.conf.json` - Configured updater
- `src-tauri/src/main.rs` - Initialized plugins
- `src/lib/store/updater.ts` - Implemented store
- `src/lib/hooks/use-soft-updater.ts` - Implemented hook
- `src/components/layout/TopBar.tsx` - Added UI integration
- `src/components/SettingsDialog.tsx` - Added settings section

## Conclusion

The Tauri v2 soft updater is now fully integrated into Osskins. Users will automatically receive notifications about new versions and can update with a single click. The system is secure, user-friendly, and leverages GitHub Releases as the update distribution mechanism.
