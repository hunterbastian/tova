Original prompt: Propose AND implement one high-leverage viral feature for my app.

- Added share UI overlay with button + toast in index.html.
- Implemented share flow with Web Share API + clipboard/download fallback and analytics events in src/main.js.
- Added render_game_to_text and advanceTime hooks for deterministic testing.
- Added fullscreen toggle (f key).

- Attempted Playwright run; missing playwright package. npm install failed due to registry network resolution error.
- Dev server requires escalated permissions; started via approved npm run dev (check /tmp/tova-dev.log).
