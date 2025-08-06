# Branch Protection Setup Guide

This guide explains how to set up branch protection rules for your RustBot repository to prevent direct pushes to `main` and require all CI checks to pass before merging.

## GitHub UI Setup (Recommended)

### Step 1: Navigate to Branch Protection Settings
1. Go to your repository on GitHub
2. Click the **Settings** tab
3. In the left sidebar, click **Branches**
4. Click **Add rule** button

### Step 2: Configure Branch Protection Rule
In the "Branch name pattern" field, enter: `main`

### Step 3: Enable Required Settings
Check the following boxes:

#### Pull Request Requirements
- ✅ **Require a pull request before merging**
- ✅ **Require approvals** (set to 1 or more reviewers)
- ✅ **Dismiss stale PR approvals when new commits are pushed**
- ✅ **Require review from code owners** (optional, if you have a CODEOWNERS file)

#### Status Check Requirements
- ✅ **Require status checks to pass before merging**
- ✅ **Require branches to be up to date before merging**

In the status checks section, add these required checks:
- `Test` (from ci.yml workflow)
- `Security Audit` (from ci.yml workflow)
- `Docker Build Test` (from ci.yml workflow)

#### Push Restrictions
- ✅ **Restrict pushes that create files**
- ✅ **Include administrators** (applies rules to repository admins too)

### Step 4: Save Changes
Click **Create** to save the branch protection rule.

## GitHub CLI Setup (Alternative)

If you prefer using the command line, you can set up branch protection with the GitHub CLI:

```bash
# Make sure you're authenticated with GitHub CLI
gh auth login

# Set up branch protection for main branch
gh api repos/:owner/:repo/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["Test","Security Audit","Docker Build Test"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
  --field restrictions=null
```

Replace `:owner` and `:repo` with your actual GitHub username and repository name.

## What These Rules Accomplish

### 🚫 **No Direct Pushes to Main**
- All changes must go through pull requests
- Prevents accidental commits directly to the main branch
- Ensures code review process is followed

### ✅ **Automated Quality Checks**
- All tests must pass before merging
- Code formatting must be correct (`cargo fmt`)
- No linting warnings allowed (`cargo clippy`)
- Security vulnerabilities are checked (`cargo audit`)
- Docker build must succeed

### 👥 **Human Review Required**
- At least one approval required before merging
- Stale approvals are dismissed when new commits are pushed
- Ensures human oversight of all changes

### 🔄 **Branch Synchronization**
- Branches must be up-to-date with main before merging
- Prevents merge conflicts and ensures latest code is tested

## Workflow Integration

Your CI pipeline now includes these jobs that must pass:

1. **Test Job**: Runs `cargo test`, `cargo fmt --check`, `cargo clippy`
2. **Security Audit Job**: Runs `cargo audit` for dependency vulnerabilities
3. **Docker Build Test Job**: Ensures Docker image builds successfully

The Docker push workflow will only run after all CI checks pass.

## Testing Your Setup

1. Create a new branch: `git checkout -b test-branch-protection`
2. Make a small change and commit it
3. Try to push directly to main: `git push origin main` (this should fail)
4. Push to your branch: `git push origin test-branch-protection`
5. Create a pull request on GitHub
6. Verify that CI checks run and must pass before merge is allowed

## Troubleshooting

### Status Checks Not Appearing
- Make sure the CI workflow has run at least once
- Check that the job names in the workflow match the required status checks
- Verify the workflow is on the correct branch

### Can't Find Required Checks
- Push a commit to trigger the CI workflow first
- Wait for the workflow to complete
- Then the check names will appear in the branch protection settings

### Admin Override Not Working
- Make sure "Include administrators" is checked in branch protection settings
- Repository admins can temporarily disable branch protection if needed

## Best Practices

1. **Start with basic protection** and gradually add more restrictions
2. **Test the setup** with a dummy PR before applying to important branches
3. **Document your workflow** so team members understand the process
4. **Regular security audits** by running `cargo audit` locally
5. **Keep dependencies updated** to avoid security vulnerabilities

This setup ensures code quality, security, and proper review processes for your RustBot project!