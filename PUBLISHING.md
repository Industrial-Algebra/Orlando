# Publishing Guide

This document explains how to publish Orlando to crates.io and npm using the automated CI/CD pipeline.

## Prerequisites

1. **NPM Token**: An `NPM_TOKEN` secret must be configured in the GitHub repository
   - Go to Settings → Secrets and variables → Actions
   - Create a secret named `NPM_TOKEN` with your npm access token
   - ✅ This has already been set up

2. **Cargo Registry Token**: A `CARGO_REGISTRY_TOKEN` secret must be configured in the GitHub repository
   - Go to Settings → Secrets and variables → Actions
   - Create a secret named `CARGO_REGISTRY_TOKEN` with your crates.io API token
   - Generate a token at [crates.io/settings/tokens](https://crates.io/settings/tokens) with "publish-new" and "publish-update" scopes

3. **Permissions**: You must have push access to create tags on the repository

## Publishing Process

### Automated Publishing (Recommended)

The publish workflow automatically triggers on version tags and publishes to npm.

#### Step 1: Prepare the Release

1. **Update version numbers** (if not already done):
   ```bash
   # Update Cargo.toml
   sed -i 's/^version = .*/version = "X.Y.Z"/' Cargo.toml

   # Update package.json
   npm version X.Y.Z --no-git-tag-version
   ```

2. **Update CHANGELOG** (create one if needed):
   ```markdown
   ## [X.Y.Z] - YYYY-MM-DD

   ### Added
   - New feature 1
   - New feature 2

   ### Changed
   - Updated behavior

   ### Fixed
   - Bug fix
   ```

3. **Commit changes**:
   ```bash
   git add Cargo.toml package.json CHANGELOG.md
   git commit -m "chore: bump version to X.Y.Z"
   git push origin main
   ```

#### Step 2: Create and Push Tag

Create a version tag following semantic versioning:

```bash
# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"

# Push the tag to GitHub
git push origin vX.Y.Z
```

**Tag format**: `vMAJOR.MINOR.PATCH` (e.g., `v0.1.0`, `v1.2.3`)

#### Step 3: Monitor the Workflow

1. Go to **Actions** tab in GitHub repository
2. Watch the "Publish to npm" workflow
3. The workflow will:
   - Run all tests (unit, integration, property, clippy, fmt)
   - Build the WASM package in release mode
   - Publish to npm with public access
   - Create a GitHub Release with artifacts

#### Step 4: Verify Publication

1. **Check crates.io**:
   ```bash
   cargo search orlando-transducers
   ```

2. **Check npm**:
   ```bash
   npm view orlando-transducers
   ```

3. **Check GitHub Releases**:
   - Go to repository → Releases
   - Verify the new release is created with attached WASM files

4. **Test installations**:
   ```bash
   # Rust
   cargo add orlando-transducers@X.Y.Z

   # JavaScript
   npm install orlando-transducers@X.Y.Z
   ```

### Manual Testing (Dry Run)

You can test the publishing workflow without actually publishing:

1. Go to **Actions** → **Publish to npm**
2. Click **Run workflow**
3. Select branch: `main`
4. Enter version (e.g., `0.1.0`)
5. Click **Run workflow**

This runs the full workflow including `npm publish --dry-run` but doesn't actually publish.

## Workflow Details

### What the Workflow Does

1. **Checkout**: Clones the repository
2. **Setup**: Installs Rust, wasm-pack, and Node.js
3. **Version Update**: Updates version in Cargo.toml and package.json
4. **Testing**: Runs comprehensive test suite
   - Unit tests
   - Integration tests
   - Property tests
   - Clippy linting
   - Rustfmt check
   - WASM tests
5. **Publish to crates.io**: Publishes the Rust crate (if triggered by tag)
6. **Build**: Compiles optimized WASM package
   - Cleans pkg directory to avoid stale files
   - Builds with wasm-pack in release mode
   - Adds copyright metadata to generated package.json
7. **Publish to npm**: Publishes the npm package (if triggered by tag)
8. **Release**: Creates GitHub Release with notes and artifacts

### Workflow Triggers

- **Automatic**: On push of tags matching `v*.*.*`
- **Manual**: Via workflow_dispatch with version input (dry run)

### Environment Variables

- `CARGO_REGISTRY_TOKEN`: Set from `CARGO_REGISTRY_TOKEN` secret for crates.io authentication
- `NODE_AUTH_TOKEN`: Set from `NPM_TOKEN` secret for npm authentication

## Versioning Strategy

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version: Incompatible API changes
- **MINOR** version: New functionality (backwards compatible)
- **PATCH** version: Bug fixes (backwards compatible)

### Pre-release Versions

For alpha, beta, or RC releases:

```bash
git tag -a v0.2.0-alpha.1 -m "Release v0.2.0-alpha.1"
git push origin v0.2.0-alpha.1
```

These will publish with the `latest` tag on npm. To use a different tag:

Modify the publish workflow to add:
```bash
npm publish --tag beta
```

## Troubleshooting

### Publish Failed: Version Already Exists

**Error**: `Cannot publish over the previously published versions` (npm) or `crate version already exists` (crates.io)

**Solution**: Increment the version number and create a new tag

### Publish Failed: Authentication Error (npm)

**Error**: `401 Unauthorized`

**Solution**:
1. Verify `NPM_TOKEN` secret is correctly set
2. Ensure the token has publish permissions
3. Token may have expired - generate a new one

### Publish Failed: Authentication Error (crates.io)

**Error**: `authentication failed` or `403 Forbidden`

**Solution**:
1. Verify `CARGO_REGISTRY_TOKEN` secret is correctly set
2. Ensure the token has "publish-new" and "publish-update" scopes
3. Token may have expired - generate a new one at [crates.io/settings/tokens](https://crates.io/settings/tokens)

### Tests Failed

**Error**: Tests fail during workflow

**Solution**:
1. Run tests locally: `cargo test --all-features --target x86_64-unknown-linux-gnu`
2. Fix failing tests
3. Commit and push fixes
4. Delete and recreate the tag

### WASM Build Failed

**Error**: wasm-pack build fails

**Solution**:
1. Test locally: `wasm-pack build --release --target web`
2. Check for compilation errors in Rust code
3. Ensure all dependencies are compatible with wasm32 target

## Rollback

If you need to unpublish a version from npm:

```bash
npm unpublish orlando-transducers@X.Y.Z
```

**Warning**: npm strongly discourages unpublishing. Only do this within 72 hours of publication.

## Best Practices

1. **Test thoroughly** before tagging
2. **Update documentation** in the same release
3. **Write clear release notes** in GitHub Releases
4. **Follow semantic versioning** strictly
5. **Keep CHANGELOG** up to date
6. **Test the package** after publishing

## Example Release Workflow

```bash
# 1. Make sure you're on main and up to date
git checkout main
git pull origin main

# 2. Update versions
npm version 0.2.0 --no-git-tag-version
sed -i 's/^version = .*/version = "0.2.0"/' Cargo.toml

# 3. Update CHANGELOG
echo "## [0.2.0] - $(date +%Y-%m-%d)" >> CHANGELOG.md
echo "### Added" >> CHANGELOG.md
echo "- New feature description" >> CHANGELOG.md

# 4. Commit
git add Cargo.toml package.json CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"
git push origin main

# 5. Create and push tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0

# 6. Wait for CI/CD to complete
# 7. Verify on npm and GitHub Releases
```

## Resources

- [npm Publishing Guide](https://docs.npmjs.com/packages-and-modules/contributing-packages-to-the-registry)
- [Semantic Versioning](https://semver.org/)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
