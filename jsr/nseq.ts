/**
 * Executable JSR entrypoint for the `nseq` sequential shell command.
 *
 * @module
 */

import { runInvocation } from "./shared.ts";

if (import.meta.main) {
  await runInvocation("nseq");
}
