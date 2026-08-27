import React from "react";
import { createCliRenderer } from "@opentui/core";
import { createRoot } from "@opentui/react";
import { App } from "./app";

async function main() {
  const initialWorkspace = process.argv[2] || "Default";

  // Create native OpenTUI CLI renderer with Ctrl+C exit handler and native OS mouse selection
  const renderer = await createCliRenderer({
    exitOnCtrlC: true,
    useMouse: false,
  });

  const root = createRoot(renderer);

  const handleExit = () => {
    try {
      renderer.destroy();
    } catch (_) {}
    process.exit(0);
  };

  root.render(<App initialWorkspace={initialWorkspace} onExit={handleExit} />);
}

main().catch((err) => {
  console.error("Fatal error starting AnyContext OpenTUI:", err);
  process.exit(1);
});
