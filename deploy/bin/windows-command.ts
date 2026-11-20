// Windows PowerShell 5 removes empty environment values. Keep the empty Rust wrapper in
// the native child's environment so Cargo never tries to execute the Unix cache script.
const command = process.argv.slice(2);
if (command.length === 0) throw new Error("A native build command is required");
const child = Bun.spawn(command, {
  env: { ...process.env, RUSTC_WRAPPER: "" },
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});
process.exit(await child.exited);
