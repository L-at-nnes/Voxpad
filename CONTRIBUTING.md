# Contributing to Voxpad

Thanks for considering a contribution. Bug reports, small fixes, new ideas and documentation improvements are all welcome.

## Before you start

- **Bugs.** Search the [issues](https://github.com/L-at-nnes/Voxpad/issues) first, then open one with the bug template. Attach `data/voxpad.log` when it is relevant.
- **Features.** Open an issue before writing code for anything beyond a small change, so we can agree on scope. Voxpad is deliberately focused: a local, fast soundboard. No voice changer, no online library, no accounts.
- **Small fixes** (typos, obvious bugs, docs) can go straight to a pull request.

## Setup

See [docs/building.md](docs/building.md) for prerequisites and the dev loop, and [docs/architecture.md](docs/architecture.md) for a map of the code.

```powershell
git clone https://github.com/<you>/Voxpad.git
cd Voxpad
.\build.ps1          # full portable build, also checks your toolchain
```

## Making a change

1. Fork the repository and create a branch from `master`: `git switch -c fix/short-description`.
2. Keep the change focused. One concern per pull request, and no unrelated reformatting.
3. Follow the style of the surrounding code, including comment density and naming. Rust code should pass `cargo fmt` and `cargo clippy`; frontend code should pass `npm run lint`.
4. Add or update tests when behavior changes (`cargo test` in `src-tauri/`, `npm run test:unit` in `ui-src/`).
5. For UI changes, include a screenshot in the pull request and reuse the existing tokens and `Vx*` components in `ui-src/src/styles/voxpad.scss`.
6. User-facing strings go in `ui-src/src/locales/en.json`, never inline in components.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/), lowercase, imperative, no trailing period:

```
fix(tags): keep the active filter after deleting a sound
feat(import): support .ogg files
docs(usage): explain the queue
```

Common types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`.

## Pull requests

- Fill in the template: what changed, how you tested it.
- CI builds the portable package and runs the Rust tests; it must be green.
- Expect review comments. They are about the code, not about you.

## Good first areas

- Additional audio formats (symphonia features in `src-tauri/Cargo.toml`).
- Translations: files in `ui-src/src/locales/` are picked up automatically, but the app currently always starts in `en`, so a language setting is needed too.
- Linux polish (hotkeys and audio routing).
- Tests for the frontend store.

## License

By contributing you agree that your work is released under the project's [GPL-3.0](LICENSE) license.
