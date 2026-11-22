import { readFile, realpath } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import vm from "node:vm";

// Parse and link browser modules without evaluating top-level code, importing
// Node packages, fetching URLs, or instantiating WebAssembly. Dynamic import()
// targets are not discovered; the generated runtime is an explicit entry.
const entries = ["main.js", "host.js", "phone-browser.js"];
try {
  if (!process.argv[2]) throw new Error("Usage: node --experimental-vm-modules scripts/verify-site.mjs <site>");
  if (typeof vm.SourceTextModule !== "function") throw new Error("Run Node with --experimental-vm-modules.");
  const root = await realpath(path.resolve(process.argv[2]));
  const context = vm.createContext(Object.create(null));
  const modules = new Map();

  async function load(filename) {
    const file = await realpath(filename);
    const relative = path.relative(root, file);
    if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
      throw new Error(`Module escapes the packaged site: ${filename}`);
    }
    if (!modules.has(file)) {
      modules.set(file, readFile(file, "utf8").then(source => new vm.SourceTextModule(source, {
        context, identifier: pathToFileURL(file).href,
      })));
    }
    return modules.get(file);
  }

  const resolve = (specifier, referencing) => {
    if (!specifier.startsWith("./") && !specifier.startsWith("../")) {
      throw new Error(`Non-relative import ${JSON.stringify(specifier)} in ${referencing.identifier}`);
    }
    return load(fileURLToPath(new URL(specifier, referencing.identifier)));
  };

  for (const entry of entries) {
    const module = await load(path.join(root, entry));
    try {
      if (module.status === "unlinked") await module.link(resolve);
      if (module.status === "errored") throw module.error;
    } catch (error) {
      throw new Error(`${entry}: ${error.message}`);
    }
  }
  console.log(`Linked ${modules.size} local modules from ${entries.join(", ")}; no code executed.`);
} catch (error) {
  console.error(`Static site verification failed: ${error.message}`);
  process.exitCode = 1;
}
