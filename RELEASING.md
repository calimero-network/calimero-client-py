# Releasing

How a change in calimero-client-py becomes a release. Agents and people read
this before committing, titling a pull request, or releasing. The first part is
the set of rules every calimero-network repository that ships releases shares;
the second is what is specific to this repository.

<!-- shared-release-rules:start -->
## Shared rules

These rules are the same, word for word, in every calimero-network repository
that ships releases. The canonical copy is in
[core's RELEASING.md](https://github.com/calimero-network/core/blob/master/RELEASING.md);
change it there first, then copy the text between the `shared-release-rules`
markers into each repository unchanged. What differs per repository (branches,
tags, where things are published, manual steps) is in the repository section.

### The PR title is the release commit

Pull requests are squash-merged, so the PR title becomes the single commit that
lands on the default branch, and that commit is what release automation reads.
Commits inside the PR do not count. Write the title as a
[Conventional Commit](https://www.conventionalcommits.org/), lower case:
`<type>(<optional scope>): <subject>`.

Where a repository releases from its commits (semantic-release; the repository
section says whether it does), the type picks the version bump:

| PR title | Release |
| --- | --- |
| `feat: ...` | minor |
| `fix:`, `perf:` | patch |
| `docs:`, `chore:`, `refactor:`, `style:`, `test:`, `build:`, `ci:`, `revert:` | none |
| `<type>!: ...`, or a `BREAKING-CHANGE:` footer | major |

- Only the hyphenated `BREAKING-CHANGE:` footer keyword makes a major (besides
  `!`). The commit parser matches note keywords case-insensitively at the start
  of any line of the message, so with the default keyword `BREAKING CHANGE` a
  body line that happened to wrap onto "breaking change ..." published
  mero-react 7.0.0 from a commit that touched only CI and a test fixture. The
  `.releaserc.json` files therefore set `parserOpts.noteKeywords` to
  `["BREAKING-CHANGE"]`, and prose that says "breaking change" does nothing.
- Prefer `!` in the title over the footer: the title is visible in review,
  while the squash commit's body is whatever the merge dialog fills in.
- Only `feat:`, `fix:` and `perf:` release (plus a major from `!` or the
  footer); every other type releases nothing. A change to what users read in
  the package (a README shipped to npm, docstrings) that should reach them is
  titled `fix(docs):` (a patch). To ship a change that touches no feature or
  fix (e.g. a dependency bump consumers need), title it `fix(deps): ...`.
- Choose the type for what the change does to whoever depends on the release,
  not for the size of the diff.

### What counts as breaking

For a library (an SDK, a client, a package other code imports), a change is
breaking when code that built and worked against the previous release can stop
building or behave differently:

- an export, type, method, function or field is removed or renamed;
- a signature changes (a parameter added without a default, a parameter or
  return type changed, a call that becomes async);
- a field of a request, option or config type becomes required;
- accepted values narrow (an enum loses a member, validation refuses input it
  used to accept);
- the error a caller can catch or match on changes type or code.

Additions (a new export, method or optional field) are `feat`.

For an app or a service (a node, an image, a deployed backend), breaking is
what its users or operators must act on: an API route or field removed or
changed, stored data or a wire format that the previous release cannot read
or that cannot be read back, configuration or flags that must change, clients
that must upgrade first. Core declares these through its
[compatibility rules](https://github.com/calimero-network/core/blob/master/docs/src/content/docs/operate/compatibility.mdx)
(deprecations, hard cutovers), not through the title alone.

### Releases happen on merge, through automation

- A repository's workflows release it, triggered by a merge to the release
  branch (or, where the repository section says so, by a tag a human pushes).
  When a workflow's tag or pull request must start another workflow, it acts as
  the organization's GitHub App (`vars.ACTIONS_APP_ID` /
  `secrets.ACTIONS_APP_PRIVATE_KEY`): anything pushed with `GITHUB_TOKEN`
  starts no workflow.
- Never push a tag, create a GitHub Release, publish a package or image, or
  edit a version number by hand, unless the repository section names that
  step as manual and a human asked for it.
- AI agents never do any of these without a human's explicit approval, even
  when the repository section lists the step. An agent's part ends at a pull
  request with the right title.
- A bad release is fixed by the next release (a `fix:` pull request), not by
  moving or deleting a tag: npm, PyPI, Maven Central and crates.io never take
  a published version back.

### OpenAPI sync pull requests from core

The SDKs generate their HTTP clients from core's `docs/public/openapi.json`.
When it changes on core's master, core's
[`sdk-openapi-sync.yml`](https://github.com/calimero-network/core/blob/master/.github/workflows/sdk-openapi-sync.yml)
has each SDK re-vendor it on the machine-owned branch `chore/core-openapi` and
open (or refresh) one pull request. Core's classifier titles that pull request
from what changed in the generated surface: `feat(api)!:` when something a
caller uses was removed or changed, `feat(api):` when something was only
added, `chore(api):` otherwise (descriptions only), which releases nothing:
that change reaches consumers with the SDK's next release.

Don't retitle these down: `feat(api)!:` turned into `feat(api):` ships a
breaking change as a minor to every consumer on a `^` range. Retitle up if the
classifier missed a break. Don't push to `chore/core-openapi`; the workflow
force-pushes it. Fix the document or the generator in core instead.

### A major stops the fleet's automatic bumps

When a package is published, its repository's fleet-bump workflow opens a pull
request in every consumer listed in its `.github/fleet.json`.
`bump-fleet.sh` skips a jump across a major version unless the run sets
`allow_major` (core's fleet bump skips majors the same way), because such a bump
would arrive red on every release. So after a major, consumers stay on the old
major and stop receiving that package's releases until someone moves them on
purpose: plan the consumers' migration pull requests as part of shipping a
major, and expect an accidental major to cut the whole fleet off silently.

### Pre-release lines

Some repositories also publish pre-releases (core's `X.Y.Z-rc.N`, the `rc`
channel of calimero-client-py). Which branch makes which kind of release, and
how a pre-release line becomes a final, is in the repository section. Promoting
a line is a human's decision.

### Before you commit, title a pull request, or release

1. The title is a Conventional Commit whose type matches the table above and
   the breaking rules.
2. A breaking change has `!` in the title and says in the PR body what callers
   must change; where the repository keeps a hand-written changelog, the PR
   adds its entry.
3. Version fields, tags and release configuration are left alone unless that is
   the point of the PR and a human asked for it.
4. After the merge, check that the release landed as the repository section
   describes.
<!-- shared-release-rules:end -->

## This repository: calimero-client-py

Releases are made by merging to master. Nobody edits a version.

1. **Merge.** Every push to master runs `.github/workflows/release-on-merge.yml`.
   It runs semantic-release (`.releaserc.json`) over the conventional commits
   since the last release.
   - Each commit type counts as follows:
     - `feat` makes a minor release;
     - `fix` and `perf` make a patch;
     - `docs`, `chore`, `refactor`, `style`, `test`, `build`, `ci` and
       `revert` make no release;
     - `type!:` or a `BREAKING-CHANGE:` footer makes a major.
   - When the commits make a release, semantic-release pushes the tag
     `v<version>` and creates the GitHub Release, with notes from those
     commits.
2. **Build.** The tag starts `.github/workflows/build-wheels.yml`.
   - `scripts/set_version.py` writes the tag's version into `Cargo.toml` and
     `Cargo.lock` inside each build job.
   - So the sdist and the five wheels carry that version, and it is never
     committed.
   - The smoke test checks that `calimero_client_py.VERSION` equals the tag's
     version.
   - The publish job checks that every file is named with that version.
3. **Approve.** The publish job waits for the `pypi` environment's reviewers.
   Approve it in the run and the files go to PyPI.

A tag pushed by hand (`git tag v1.2.3 && git push origin v1.2.3`) is built
and published the same way.

`Cargo.toml`'s `version` on master is the last version a person wrote there.
Builds of master that are not tags report it, while the published files carry
the tag's version. `CHANGELOG.md` stays hand-written; the GitHub Release
carries the generated notes.

### Versions while 1.0 is not final

master is a pre-release branch (`"prerelease": "rc"`), so every release from
it is `1.0.0-rc.N`, which PyPI shows as `1.0.0rcN`. Commit types don't change
that: any release increments `N`. pip installs these only with `--pre` or an
exact pin.

The stable line is the `stable` branch. semantic-release refuses to run
without at least one stable branch. The name `release` cannot be used because
the old `release/v0.2.x` branches take it.

**To release 1.0.0:**

1. Move `stable` to the commit to ship: `git push origin <sha>:stable`.
2. Run *Release on merge* by hand on `stable` (Actions, Run workflow, branch
   `stable`). It tags `v1.0.0`, and the tag builds and publishes as above.
3. Then choose one:
   - Keep the shape: master makes `1.1.0-rc.N`, and `stable` makes the finals.
   - Make master the stable branch: in `.releaserc.json`, drop master's
     `prerelease` and `channel`, and remove the `stable` entry.

### Set up once

semantic-release finds the last release from its tag and from a git note that
names the tag's channel. Releases up to 1.0.0rc4 were made by hand, with
neither. Before the first merge that releases, record rc4 and create
`stable`:

```bash
git fetch origin
git tag v1.0.0-rc.4 c46c737            # the commit 1.0.0rc4 was published from
git notes --ref semantic-release add -m '{"channels":["rc"]}' v1.0.0-rc.4
git push origin v1.0.0-rc.4 refs/notes/semantic-release
git push origin c46c737:refs/heads/stable
```

Pushing that tag starts build-wheels.yml for `1.0.0rc4`, which is already
on PyPI: reject the pending `pypi` deployment, or approve it and PyPI refuses
the files. Either way nothing changes. Without these
refs, release-on-merge.yml stops before it tags anything and says which one
is missing.

The workflow needs the organization's GitHub App installed on this
repository. It reads the App's credentials from `vars.ACTIONS_APP_ID` and
`secrets.ACTIONS_APP_PRIVATE_KEY`, and the token it mints is scoped to
contents write. The App is needed because a tag pushed with `GITHUB_TOKEN`
would not start build-wheels.yml.

### Manual steps that need a human

- Approving the `pypi` environment's deployment in each `Build and Publish
  Wheels` run on a tag: nothing reaches PyPI without it.
- Releasing 1.0.0 from `stable`, the one-time setup above, and any tag pushed
  by hand. Agents never do these.

### Checking that a release landed

- The tag `vX.Y.Z` (or `vX.Y.Z-rc.N`) and its GitHub Release exist.
- The `Build and Publish Wheels` run on the tag is green, including the publish
  job after its approval.
- [PyPI](https://pypi.org/project/calimero-client-py/#history) lists the version
  (`X.Y.ZrcN` for a pre-release) with the sdist and five wheels;
  `pip install --pre calimero-client-py==<version>` installs it.

### How consumers are bumped

There is no fleet bump for this client; whoever depends on it pins it. It
follows core's API through the OpenAPI sync in the shared rules, and core's
`e2e.yml` (`clients`) runs it against every core pull request's `merod`.
