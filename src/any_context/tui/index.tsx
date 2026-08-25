import React from "react";
import { createCliRenderer } from "@opentui/core";
import { createRoot } from "@opentui/react";
import { App } from "./app";

async function main() {
  const initialWorkspace = process.argv[2] || "Default";

  // Create native OpenTUI CLI renderer with Ctrl+C exit handler
  const renderer = await createCliRenderer({
    exitOnCtrlC: true,
  });

  const root = createRoot(renderer);
  root.render(<App initialWorkspace={initialWorkspace} />);
}

main().catch((err) => {
  console.error("Fatal error starting AnyContext OpenTUI:", err);
  process.exit(1);
});
