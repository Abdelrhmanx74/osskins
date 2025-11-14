# Tauri Updater (Removed)

This simple updater guide has been archived; the soft updater feature was removed from the project.

---

## Step 1: Generate Your Keys (With Password)

Open your terminal and run:

```bash
pnpm tauri signer generate -- -w ~/.tauri/osskins.key
```

**On Windows**, use:
```bash
pnpm tauri signer generate -- -w C:\Users\YourName\.tauri\osskins.key
```

Replace `YourName` with your actual Windows username (e.g., `Mana`).

### What Happens:
1. You'll be prompted to enter a password
2. You'll be asked to confirm the password
3. Keys are generated with password protection

### Example Output:
```
Please enter a password to protect the secret key.
Password: ********
Password (one more time): ********

Deriving a key from the password in order to encrypt the secret key... done

Your keypair was generated successfully
Private: ~/.tauri/osskins.key (Keep it secret!)
Public: ~/.tauri/osskins.key.pub
```

**⚠️ IMPORTANT:** 
- Choose a password you won't forget
- Write it down securely
- You'll need this password for GitHub Secrets

---

## Step 2: Copy Your Public Key

### On Mac/Linux:
```bash
cat ~/.tauri/osskins.key.pub
```

### On Windows (PowerShell):
```powershell
Get-Content C:\Users\YourName\.tauri\osskins.key.pub
```

### On Windows (Command Prompt):
```cmd
type C:\Users\YourName\.tauri\osskins.key.pub
```

**Copy the entire output.** It looks like this:
```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDYxNzQyQjU2QUMxOTE3QTMKV1dTakZ4bXNWaXQwWVJTR2crb24yUlAwWHlVVDRTZWhSME9VK0NRMCtTdEtnc2VDbURHbHIrekkK
```

**IMPORTANT:** Copy the ENTIRE line - it's one long string with no spaces or line breaks!

---

## Step 3: Update tauri.conf.json (IF NEEDED)

⚠️ **ONLY do this if you generated NEW keys!** The repo already has a public key configured.

Open `src-tauri/tauri.conf.json` and find the `"updater"` section. Replace the `"pubkey"` value with your copied public key:

```json
"updater": {
  "pubkey": "YOUR_PUBLIC_KEY_HERE",
  "endpoints": [
    "https://github.com/Abdelrhmanx74/osskins/releases/latest/download/updates.json"
  ],
  "windows": {
    "installMode": "passive"
  }
}
```

**If you're using the existing keys, skip this step!**

---

## Step 4: Copy Your Private Key

### On Mac/Linux:
```bash
cat ~/.tauri/osskins.key
```

### On Windows (PowerShell):
```powershell
Get-Content C:\Users\YourName\.tauri\osskins.key
```

### On Windows (Command Prompt):
```cmd
type C:\Users\YourName\.tauri\osskins.key
```

**Copy the entire output.** It looks like this (but MUCH longer):
```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5ekgrWTNEOHlsc0ZCMDVJNUs0ZUZqRkF4ckpYNTQ1dXpDNnpyWWJydWNRRUFBQkFBQUFBQUFBQUFBQUlBQUFBQURJcE1JZGFVR25vakRkM1pZUlkralB6TFRqOUxoQ09KZW9ZM1pCeXhLbkJ3TWdBdHliWDMrSmgvdmJHcnJaa3...
```

**IMPORTANT:** This is your SECRET key - never share it or commit it to Git!

---

## Step 5: Add GitHub Secrets

Go to: https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions

### Secret 1: Private Key

1. Click **"New repository secret"**

2. Enter the details:
   - **Name:** `TAURI_SIGNING_PRIVATE_KEY`
   - **Value:** Paste your private key (the long string from Step 4)

3. Click **"Add secret"**

### Secret 2: Password

1. Click **"New repository secret"** again

2. Enter the details:
   - **Name:** `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   - **Value:** The password you chose in Step 1

3. Click **"Add secret"**

**Both secrets are required!**

---

## Step 6: Test with v1.5-test Tag

Now let's test everything with a test release!

### 6.1: Make sure your code is committed

```bash
git status
# If you have changes, commit them:
git add .
git commit -m "Prepare for updater test"
git push
```

### 6.2: Create and push the test tag

```bash
git tag v1.5-test
git push origin v1.5-test
```

### 6.3: Watch the build

1. Go to: https://github.com/Abdelrhmanx74/osskins/actions

2. You should see a new workflow run called "Release Build"

3. Click on it to watch the progress

4. Wait for it to complete (usually 5-10 minutes)

### 6.4: Check the release

1. Go to: https://github.com/Abdelrhmanx74/osskins/releases

2. You should see a new release called "v1.5-test"

3. It should contain these files:
   - `osskins_1.5.0_x64_en-US.msi` (or similar)
   - `osskins_1.5.0_x64_en-US.msi.zip`
   - `osskins_1.5.0_x64_en-US.msi.zip.sig`
   - `updates.json`

4. Click on `updates.json` to verify it has content like:
   ```json
   {
     "version": "1.5.0",
     "date": "2024-11-13T14:00:00.000Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
         "url": "https://github.com/Abdelrhmanx74/osskins/releases/download/v1.5-test/osskins_1.5.0_x64_en-US.msi.zip"
       }
     },
     "body": "See release notes at https://github.com/Abdelrhmanx74/osskins/releases/tag/v1.5-test"
   }
   ```

---

## Troubleshooting

### Build fails with "TAURI_SIGNING_PRIVATE_KEY not set"
- You forgot to add the GitHub secret
- Go back to Step 5 and add it

### Build fails with "signature verification failed"
- The public key in `tauri.conf.json` doesn't match your private key
- Regenerate both keys and update both the config file and GitHub secret

### No .msi.zip files are created
- Make sure `"createUpdaterArtifacts": true` is in `tauri.conf.json` under `"bundle"`
- This is already set in your repo ✅

### Can't find the keys after generation
- **Mac/Linux:** `~/.tauri/osskins.key` and `~/.tauri/osskins.key.pub`
- **Windows:** `C:\Users\YourName\.tauri\osskins.key` and `C:\Users\YourName\.tauri\osskins.key.pub`

---

## What's Next?

After successful test:

1. **Delete the test tag** (optional):
   ```bash
   git tag -d v1.5-test
   git push origin :refs/tags/v1.5-test
   ```

2. **Create real releases** with proper version numbers:
   ```bash
   git tag v1.5.3
   git push origin v1.5.3
   ```

3. **Users will automatically get updates** when they open the app!

---

## Quick Reference (archived)

### File Locations
- **Private Key:** `~/.tauri/osskins.key` (Mac/Linux) or `C:\Users\YourName\.tauri\osskins.key` (Windows)
- **Public Key:** `~/.tauri/osskins.key.pub`
- **Config File:** `src-tauri/tauri.conf.json`
- **GitHub Secrets:** https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions

### Commands
```bash
# Generate keys (with password - required)
pnpm tauri signer generate -- -w ~/.tauri/osskins.key

# View public key
cat ~/.tauri/osskins.key.pub

# View private key (keep secret!)
cat ~/.tauri/osskins.key

# Create test tag
git tag v1.5-test
git push origin v1.5-test

# Create real tag
git tag v1.5.3
git push origin v1.5.3

# Delete tag locally
git tag -d v1.5-test

# Delete tag remotely
git push origin :refs/tags/v1.5-test
```

### GitHub Secrets (archived)
The updater no longer requires secrets as the feature is removed.

---

## Security Notes

✅ **DO:**
- Keep your private key safe
- Remember your password (write it down securely)
- Back up both the key and password
- Use GitHub Secrets for CI/CD

❌ **DON'T:**
- Commit the private key to Git
- Share the private key or password
- Lose the private key or password (users won't get updates!)

---

That's it! You're all set up! 🎉
