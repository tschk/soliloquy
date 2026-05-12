# Browser Chrome

Soliloquy's browser chrome is authored from the Crepuscularity template at `ui/desktop/src/lib/crepuscularity/browserChrome.crepus`.

The web desktop imports that template as raw source through `ui/desktop/src/lib/crepuscularity/browserChrome.ts`. That module exports the routes, modes, and design tokens used by `DesktopShell.svelte`, so the Svelte shell and the mac GPUI shell share one Crepuscularity contract instead of keeping separate hand-styled chrome definitions.

The current component contract is `SoliloquyBrowserChrome`. It defines the mac browser surface:

- traffic lights and workspace/status strip
- address/navigation strip
- system routes for Terminal, Files, and Settings
- Zen/sidebar, compact, split columns, split rows, and grid mode variants
- black content surface owned by RV8/Servo page rendering

For mac-native loading, `src/shell/gpui_app.rs` uses `crepuscularity-gpui` and should stay aligned with the same `SoliloquyBrowserChrome` template contract. GPUI compile fixes belong in the shell/engine lane, but changes to chrome shape, modes, routes, and design tokens should start in the `.crepus` template and then be reflected through `browserChrome.ts`.

Servo's built-in chrome remains disabled in the appliance path. The only visible browser controls should come from this Crepuscularity-authored Soliloquy chrome.
