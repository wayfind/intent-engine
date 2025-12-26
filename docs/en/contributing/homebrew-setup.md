# Homebrew Tap Setup Guide

This document explains how to set up the automated Homebrew formula updates.

## Prerequisites

1. A GitHub repository: `wayfind/homebrew-tap`
2. A Personal Access Token (PAT) with `repo` scope

## Setup Steps

### 1. Create the homebrew-tap Repository

1. Go to GitHub and create a new repository: `wayfind/homebrew-tap`
2. Initialize with the contents from `homebrew-tap-template/`:
   ```
   homebrew-tap/
   ├── README.md
   └── Formula/
       └── intent-engine.rb
   ```

### 2. Create a Personal Access Token

1. Go to GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click "Generate new token (classic)"
3. Set:
   - Note: `homebrew-tap-update`
   - Expiration: No expiration (or set a reminder)
   - Scopes: `repo` (Full control of private repositories)
4. Copy the generated token

### 3. Add the Token as a Repository Secret

1. Go to `wayfind/intent-engine` repository
2. Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Name: `HOMEBREW_TAP_TOKEN`
5. Value: (paste the PAT from step 2)
6. Click "Add secret"

### 4. Test the Workflow

You can manually trigger the workflow to test:

1. Go to Actions → "Update Homebrew Formula"
2. Click "Run workflow"
3. Enter a valid release tag (e.g., `v0.10.1`)
4. Click "Run workflow"

## How It Works

When a new release is published:

1. The `homebrew.yml` workflow is triggered
2. It downloads the release tarballs
3. Calculates SHA256 checksums for each platform
4. Generates an updated formula
5. Pushes the updated formula to `wayfind/homebrew-tap`

## Manual Update (Alternative)

If automated updates fail, you can manually update:

```bash
# Run the update script
./scripts/update-homebrew-formula.sh 0.10.1

# Then manually copy to homebrew-tap repo
cp homebrew/intent-engine.rb /path/to/homebrew-tap/Formula/
cd /path/to/homebrew-tap
git add Formula/intent-engine.rb
git commit -m "Update intent-engine to 0.10.1"
git push
```

## Troubleshooting

### Token Permission Error

If you see permission errors:
- Verify the PAT has `repo` scope
- Check that `HOMEBREW_TAP_TOKEN` secret is set correctly
- Ensure the PAT hasn't expired

### SHA256 Mismatch

If formula installation fails with SHA256 mismatch:
- Check that the release assets are fully uploaded
- Wait a few minutes and try again
- Manually re-run the workflow

### Formula Syntax Error

If Homebrew reports formula syntax errors:
- Check the generated formula in `homebrew-tap/Formula/intent-engine.rb`
- Validate with: `brew audit --new-formula Formula/intent-engine.rb`
