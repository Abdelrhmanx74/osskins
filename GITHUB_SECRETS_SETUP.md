# 🔐 EXACT GITHUB SECRETS CONFIGURATION

## Copy these EXACT values to GitHub Secrets

Go to: https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions

---

## 1️⃣ TAURI_SIGNING_PRIVATE_KEY

**Name:** `TAURI_SIGNING_PRIVATE_KEY`

**Value (copy EXACTLY as shown below):**

```
dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5ekgrWTNEOHlsc0ZCMDVJNUs0ZUZqRkF4ckpYNTQ1dXpDNnpyWWJydWNRRUFBQkFBQUFBQUFBQUFBQUlBQUFBQURJcE1JZGFVR25vakRkM1pZUlkralB6TFRqOUxoQ09KZW9ZM1pCeXhLbkJ3TWdBdHliWDMrSmgvdmJHcnJaa3NyZkc4MXdGLzIwUlhwSS92Zzl1K2VDMFBIdVJjYi9jb2tvMjJVeGtSNWs5U3ZZZDViSTZGaUc4amJtNE9PU3Y1Rkd0MEJ4T3MrNVU9Cg==
```

---

## 2️⃣ TAURI_SIGNING_PRIVATE_KEY_PASSWORD

**Name:** `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

**Value:** The password you entered when generating the keys (you should remember this)

---

## ✅ Local Configuration (Already Done)

Your `src-tauri/tauri.conf.json` should have this public key:

```json
"updater": {
  "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEM2RDk1RUFGNkRFQkZBNTMKUldSVCt1dHRyMTdaeHNTVG5ackxEVFZkL1hydFdJYW9EO DBpUUxEOVJMSUJNTlhOR1JYeHMrMW8K",
  "endpoints": [
    "https://github.com/Abdelrhmanx74/osskins/releases/download/v{version}/updates.json"
  ],
  "windows": {
    "installMode": "passive"
  }
}
```

This is **ALREADY CORRECT** in your local file! ✅

---

## 📋 Steps to Update GitHub Secrets

### Step 1: Delete or Update Old Secret
1. Go to https://github.com/Abdelrhmanx74/osskins/settings/secrets/actions
2. Find `TAURI_SIGNING_PRIVATE_KEY`
3. Click "Update" (or "Remove" then "New repository secret")

### Step 2: Add New Private Key
1. Name: `TAURI_SIGNING_PRIVATE_KEY`
2. Value: **Copy the entire string from section 1️⃣ above**
   - No extra spaces
   - No line breaks
   - Just the base64 string starting with `dW50cnVzdGVk...`

### Step 3: Add Password Secret
1. Click "New repository secret"
2. Name: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
3. Value: Your password (the one you created when running `pnpm tauri signer generate`)

---

## ⚠️ CRITICAL NOTES

1. **DO NOT** use the `private_ed25519.pem` file - that's the WRONG format!
2. **DO NOT** add extra spaces or line breaks when copying the key
3. **DO NOT** commit these keys to Git
4. **Keep your password safe** - without it, you can't sign updates

---

## 🧪 Testing After Setup

1. Push your code to GitHub
2. Create a new tag: `git tag v1.5.3 && git push origin v1.5.3`
3. GitHub Actions will automatically build and sign your release
4. Check the workflow at: https://github.com/Abdelrhmanx74/osskins/actions

---

## 🎯 Summary

- **Local pubkey**: ✅ Already correct in `tauri.conf.json`
- **GitHub private key**: Update with value from section 1️⃣
- **GitHub password**: Set to your password
- **Old PEM file**: ❌ Ignore it - wrong format!
