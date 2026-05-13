# Browser Chrome

Soliloquy's browser chrome is authored from the Crepuscularity template at `ui/desktop/src/lib/crepuscularity/browserChrome.crepus`.

The web desktop imports that template as raw source through `ui/desktop/src/lib/crepuscularity/browserChrome.ts`. That module exports the routes, modes, and design tokens used by `DesktopShell.svelte`.

The current component contract is `SoliloquyBrowserChrome`. It defines the mac browser surface:

- traffic lights and workspace/status strip
- address/navigation strip
- system routes for Terminal, Files, and Settings
- Zen/sidebar, compact, split columns, split rows, and grid mode variants
- black content surface owned by RV8/Servo page rendering

For mac-native loading, use `tools/soliloquy/start_macos.sh`. That path launches browser-only Servo with `--no-browser-chrome`. It must not start `sold` or serve the Svelte appliance chrome. The old GPUI chrome is not the supported browser-engine path.

Servo's built-in chrome remains disabled in the appliance path. The only visible browser controls should come from this Crepuscularity-authored Soliloquy chrome.
