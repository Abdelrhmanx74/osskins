# Tauri v2 Soft Updater - UI Documentation

## User Interface Overview

This document describes the user interface elements added for the Tauri v2 soft updater.

## 1. TopBar Menu Integration

### Menu Button with Update Badge
**Location:** Top-right corner of the application window

**Visual Elements:**
- Menu button (hamburger icon) with three horizontal lines
- When update is available: Small pulsing red dot badge on top-right corner of button
- Badge color: Primary theme color with pulse animation

**Behavior:**
- Badge appears when `updateStatus === "available"`
- Badge disappears after user checks or dismisses update

### Menu Items

#### "Check for App Updates" Menu Item
**Visual Elements:**
- Download icon (arrow pointing down into box)
- Text: "Check for App Updates"
- When update available: "New" badge in primary color on the right

**Behavior:**
- Clicking opens the AppUpdateDialog
- Shows current update status

#### "Release Notes" Menu Item
**Visual Elements:**
- GitCommit icon (branching diagram)
- Text: "Release Notes"

**Behavior:**
- Clicking opens the ReleaseNotesDialog
- Fetches and displays release history from GitHub

## 2. AppUpdateDialog

### Dialog States

#### State: Idle (Initial)
**Visual Elements:**
- Title: "App Updates"
- Description: "Keep your app up to date with the latest features and fixes."
- Large RefreshCw icon (refresh/sync) centered
- Heading: "Check for Updates"
- Description: "Click the button below to check for available updates."
- Buttons: "Cancel" (outline), "Check for Updates" (primary with RefreshCw icon)

#### State: Checking
**Visual Elements:**
- Large spinning RefreshCw icon (animated)
- Text: "Checking for updates..."
- Button: "Checking..." (disabled)

#### State: Available
**Visual Elements:**
- Sparkles icon with primary color
- Heading: "New version available!"
- Version info: "Version X.X.X is now available. You are currently on version Y.Y.Y."
- Release date: "Released on [Date]"
- Separator line
- Release Notes section:
  - Heading: "Release Notes"
  - Scrollable text area with muted background
  - Pre-formatted text showing changelog
- Buttons: "Later" (outline), "Download Update" (primary with Download icon)

#### State: Downloading
**Visual Elements:**
- Download icon (animated pulse)
- Heading: "Downloading update..."
- Description: "Please wait while the update is being downloaded."
- Progress bar showing percentage (0-100%)
- Progress text: "X% complete"
- Button: "Downloading..." (disabled)

#### State: Downloaded
**Visual Elements:**
- Large green checkmark icon in circular background
- Heading: "Update downloaded successfully!"
- Description: "Click 'Install & Restart' to complete the update."
- Buttons: "Later" (outline), "Install & Restart" (primary with RefreshCw icon)

#### State: Up-to-date
**Visual Elements:**
- Large green checkmark icon in circular background
- Heading: "You're up to date!"
- Description: "You are running the latest version (X.X.X)."
- Button: "Close" (primary)

#### State: Error
**Visual Elements:**
- Large red AlertCircle icon in circular background
- Heading: "Update failed"
- Error message displayed in muted text
- Buttons: "Close" (outline), "Try Again" (primary with RefreshCw icon)

### Dialog Dimensions
- Width: `sm:max-w-md` (medium dialog)
- Content padding: Standard dialog padding
- Maximum height for release notes: `max-h-48` with scroll

## 3. ReleaseNotesDialog

### Visual Elements

**Header:**
- Title: "Release Notes" with GitCommit icon
- Description: "View release history and changelog for Osskins"

**Content Area:**
- Scrollable area: `max-h-[60vh]` (60% of viewport height)
- Padding: `pr-4` (right padding for scrollbar)

**For each release:**

1. **Version Badge Section:**
   - Version tag badge with Tag icon
   - "Current" badge (secondary) if this is the current version
   - "Pre-release" badge (secondary) if applicable
   - External link icon (links to GitHub release page)

2. **Release Info:**
   - Release name as heading (font-semibold)
   - Calendar icon with release date

3. **Release Notes:**
   - Muted background container
   - Pre-formatted text with proper wrapping
   - Scrollable if content is long

4. **Separator:** Horizontal line between releases

**Loading State:**
- Centered text: "Loading releases..."

**Error State:**
- Error message in destructive color
- "Retry" button (outline, small)

**Empty State:**
- Centered text: "No releases found"

**Footer:**
- "Close" button (outline)

### Dialog Dimensions
- Width: `sm:max-w-2xl` (large dialog)
- Maximum height: `max-h-[80vh]` (80% of viewport height)

## 4. SettingsDialog Integration

### Application Updates Section
**Location:** Left column of Settings dialog, after "Import/Export Skins" section

**Visual Elements:**

1. **Section Header:**
   - Label: "Application Updates"

2. **Current Version Display:**
   - Muted background container with rounded corners
   - Left side:
     - Text: "Current Version" (font-medium)
     - Version number in muted text
   - Right side (when update available):
     - Badge: "Update Available" with Download icon

3. **Action Buttons (Side by Side):**
   - "Check for Updates" button (secondary, flex-1) with Download icon
   - "Release Notes" button (outline, flex-1) with GitCommit icon

4. **Help Text:**
   - Small muted text: "Updates are automatically checked when the app starts."

### Visual Flow
```
┌─────────────────────────────────────┐
│ Application Updates                 │
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │ Current Version      [New Badge]│ │
│ │ 1.5.2                           │ │
│ └─────────────────────────────────┘ │
│                                     │
│ [Check for Updates] [Release Notes]│
│                                     │
│ Updates are automatically checked...│
└─────────────────────────────────────┘
```

## 5. Color Scheme

### Status Colors
- **Available/New:** Primary theme color
- **Success/Downloaded:** Green (`text-green-500`, `bg-green-500/20`)
- **Error:** Red (`text-red-500`, `bg-red-500/20`)
- **Info/Default:** Muted (`text-muted-foreground`)

### Animations
- **Badge Pulse:** `animate-pulse` on update notification badge
- **Download Icon:** `animate-pulse` during download
- **Spinner:** `animate-spin` during checking

## 6. Responsive Design

All dialogs are responsive:
- Mobile: Full-width with proper padding
- Desktop: Fixed max-width with centered positioning
- Content areas are scrollable when content exceeds viewport

## 7. Accessibility

- All interactive elements have proper labels
- Keyboard navigation supported
- Screen reader friendly with semantic HTML
- Focus states on all buttons
- ARIA attributes via Radix UI primitives

## 8. User Flow Diagram

```
[App Starts]
    ↓
[Auto-check for updates (silent)]
    ↓
[Update available?] ─NO→ [Continue normally]
    ↓ YES
[Show badge on menu button]
    ↓
[User clicks menu] → [User sees "New" badge on update item]
    ↓
[User clicks "Check for App Updates"]
    ↓
[AppUpdateDialog opens] → [Shows update info]
    ↓
[User clicks "Download Update"]
    ↓
[Progress bar shows download]
    ↓
[Download completes]
    ↓
[User clicks "Install & Restart"]
    ↓
[App installs and relaunches]
    ↓
[User now on new version]
```

## 9. Alternative Flow: Release Notes

```
[User opens Settings or TopBar menu]
    ↓
[User clicks "Release Notes"]
    ↓
[ReleaseNotesDialog opens]
    ↓
[Fetches releases from GitHub API]
    ↓
[Shows release history with versions, dates, notes]
    ↓
[User can scroll through releases]
    ↓
[User can click external link to view on GitHub]
    ↓
[User closes dialog]
```

## 10. Toast Notifications

The updater shows toast notifications for user feedback:

- **Update available (manual check):** Success toast with version number
- **No update available (manual check):** Info toast "You're up to date!"
- **Download started:** Automatic, shown in dialog
- **Download complete:** Success toast "Update downloaded successfully!"
- **Install starting:** Info toast "Restarting application..."
- **Error:** Error toast with descriptive message

## Summary

The updater UI is designed to be:
- **Non-intrusive:** Auto-checks silently, shows subtle badge
- **Informative:** Clear status messages and progress indicators
- **Actionable:** One-click operations with clear CTAs
- **Professional:** Consistent with app's design language
- **Accessible:** Keyboard navigation and screen reader support
- **Responsive:** Works on all screen sizes

All UI elements use the existing Shadcn/Radix UI component library for consistency with the rest of the application.
