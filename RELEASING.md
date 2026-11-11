# Releasing

Releases are made by merging to master. Nobody edits a version.

1. **Merge.** Every push to master runs `.github/workflows/release-on-merge.yml`.
   It runs semantic-release (`.releaserc.json`) over the conventional commits
   since the last release.
   - Each commit type counts as follows:
     - `feat` makes a minor release;
     - `fix`, `chore`, `docs`, `refactor` and `perf` make a patch;
     - `ci`, `test` and `build` make no release;
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

## Versions while 1.0 is not final

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

## Set up once

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
