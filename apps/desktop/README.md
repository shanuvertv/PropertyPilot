# PropertyPilot desktop

Tauri 2 shell (`src-tauri/`) around the React UI (`src/`). The UI talks to
`renewal-server` over HTTP; the shell only adds the secure store for the server
address/token (Windows Credential Manager via `keyring`) and native notifications.

```bash
npm install
npm run dev          # UI only in a browser at http://localhost:1420
npm run tauri dev    # full desktop app with hot reload
npm run tauri build  # MSI + NSIS installers under ../../target/release/bundle/
npm run typecheck && npm test
```

See the repository README for server setup.
