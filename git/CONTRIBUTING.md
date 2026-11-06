# Contributing

[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)

Guidelines for contributing to Coding Assistants.

---

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/Coding-Assistants.git
   cd Coding-Assistants
   ```
3. Set up the development environment (see [DEVELOPMENT.md](DEVELOPMENT.md))
4. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

---

## Development Workflow

### Before Writing Code

1. **Check existing issues** -- Someone may already be working on it
2. **Open an issue first** for significant changes to discuss the approach
3. **Read the architecture docs** ([ARCHITECTURE.md](ARCHITECTURE.md)) to understand the system

### Making Changes

1. Follow the existing code style (see [Code Standards](#code-standards) below)
2. Keep changes focused -- one feature or fix per branch
3. Update documentation if your change affects:
   - IPC commands (update `AGENTS.md` command table)
   - Dependencies (update `DEPENDENCIES.md`)
   - Architecture (update `ARCHITECTURE.md`)
   - Security-relevant behavior (update `SECURITY.md`)

### Testing Your Changes

```bash
# Frontend type check
npx tsc --noEmit

# Rust compile check
cd src-tauri && cargo check

# Rust linting
cd src-tauri && cargo clippy

# Full app smoke test
npm run tauri dev
```

See [TESTING.md](TESTING.md) for the full testing strategy.

---

## Commit Messages

Use clear, descriptive commit messages:

```
<type>: <short description>

<optional longer description>
```

### Types

| Type       | Use For                                     |
| ---------- | ------------------------------------------- |
| `feat`     | New feature                                 |
| `fix`      | Bug fix                                     |
| `refactor` | Code restructuring (no behavior change)     |
| `docs`     | Documentation changes                       |
| `style`    | Formatting, whitespace (no logic change)    |
| `test`     | Adding or updating tests                    |
| `chore`    | Build config, dependencies, tooling         |

### Examples

```
feat: add temperature parameter to LLM provider config

fix: prevent path traversal in resource file reads

docs: update ARCHITECTURE.md with TCP server protocol details

refactor: extract ModelSelect into separate component
```

---

## Pull Request Process

1. **Update your branch** with the latest `main`:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

2. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

3. **Open a Pull Request** against `main` with:
   - Clear title describing the change
   - Description of what changed and why
   - Link to related issue(s) if applicable
   - Screenshots for UI changes

4. **Address review feedback** -- Push additional commits to the same branch

5. **Merge** -- Maintainers will merge after approval

### PR Checklist

- [ ] Code compiles without errors (`cargo check`, `npx tsc --noEmit`)
- [ ] Rust linting passes (`cargo clippy`)
- [ ] New Tauri commands are registered in the invoke handler
- [ ] IPC types are consistent between TypeScript and Rust
- [ ] No secrets or API keys are committed
- [ ] Documentation is updated for user-facing changes
- [ ] Security checklist reviewed (see [SECURITY.md](SECURITY.md))

---

## Code Standards

### Rust (`src-tauri/src/`)

- **Formatting**: Use `rustfmt` defaults
- **Linting**: Pass `cargo clippy` without warnings
- **Error handling**: Use `Result<T, String>` for Tauri commands
- **Async**: Use `async fn` with Tokio for I/O operations
- **Serialization**: Derive `Serialize` + `Deserialize` for IPC types
- **Naming**: snake_case for functions and variables, PascalCase for types

### TypeScript (`src/`)

- **Strict mode**: Enabled -- no implicit `any`, no unused variables
- **Target**: ES2020
- **Module**: ESNext
- **JSX**: `react-jsx` transform (no React import needed)
- **Types**: Use interfaces for IPC payloads, match Rust struct field names exactly

### CSS (`src/index.css`)

- Use CSS custom properties for theme colors
- Follow the glass-morphism design pattern
- Keep specificity low -- prefer class selectors
- Use the existing spacing and sizing conventions

---

## Architecture Guidelines

### Adding New Features

1. **UI-only changes**: Modify `src/App.tsx` and `src/index.css`
2. **New Tauri command**: Add to `src-tauri/src/lib.rs`, register in handler, call from frontend
3. **New module**: Create a new `.rs` file in `src-tauri/src/`, expose via `lib.rs`
4. **New LLM provider**: Extend `llm_client.rs`, update frontend dropdown

### IPC Contract

Every change to the IPC boundary must keep both sides in sync:

```
Frontend invoke() <----> Backend #[tauri::command]
  TypeScript types <----> Rust serde structs
```

If you change a Rust command signature, update the corresponding `invoke()` call immediately.

### Security Considerations

Before submitting:

- No shell invocations (`sh -c`, `bash -c`, `cmd /c`)
- File paths validated and sandboxed to workspace
- User input not interpolated into command arguments
- New permissions documented in capabilities

See the full checklist in [SECURITY.md](SECURITY.md).

---

## Reporting Issues

### Bug Reports

Include:
- Operating system and version
- Node.js, Rust, and Tauri CLI versions
- Steps to reproduce
- Expected vs actual behavior
- Terminal output and browser console errors

### Feature Requests

Include:
- Description of the desired feature
- Use case / motivation
- Proposed approach (if any)

Use the `enhancement` label for feature requests.

---

## License

By contributing, you agree that your contributions will be licensed under the [GNU Affero General Public License v3.0](LICENSE).
