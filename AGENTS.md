# AGENTS.md

This file documents the standard developer and game workflows for this repo.

## Standard Web Workflows

### Install

```sh
npm install
```

### Dev Server

```sh
npm run dev
```

### Build

```sh
npm run build
```

### Preview Production Build

```sh
npm run preview
```

### Tests / Lint / Format

No test, lint, or format scripts are defined in `package.json` yet.

## Game Workflows (Godot)

There is a Godot project under `godot_backup/`.

### Open In Godot Editor (GUI)

- Open Godot and select the project at `godot_backup/`.
- Press Play in the editor to run.

### Launch Editor From CLI (Optional)

If `godot` is on your PATH:

```sh
godot --editor --path godot_backup
```

### Export Builds (Editor)

- Configure export presets in Godot.
- Use **Project > Export** to build.

