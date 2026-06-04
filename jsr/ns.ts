/**
 * Executable JSR entrypoint for the `ns` shell command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("ns");
}
