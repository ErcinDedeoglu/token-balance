const KIT = new Set([
  "docs",
  "memory",
  "openspec",
  "tooling",
  ".github",
  ".git",
  ".opencode",
  ".claude",
  ".githooks",
  "node_modules",
  "dist",
  "build",
])

function topDir(filePath, root) {
  if (!filePath) return null
  const rel = String(filePath).replace(/\\/g, "/").replace(root.replace(/\\/g, "/") + "/", "")
  if (!rel.includes("/")) return null
  const top = rel.split("/")[0]
  if (KIT.has(top)) return null
  return top
}

export default async ({ directory }) => {
  const fs = await import("node:fs")
  const path = await import("node:path")
  const { spawnSync } = await import("node:child_process")
  const checker = path.join(directory, "tooling/agent-kit/check.sh")

  function runCheck() {
    if (!fs.existsSync(checker)) return { status: 0, text: "" }
    const r = spawnSync("bash", [checker, directory], { encoding: "utf8", cwd: directory })
    return { status: r.status ?? 1, text: `${r.stdout || ""}${r.stderr || ""}` }
  }

  return {
    "tool.execute.before": async (input, output) => {
      const tool = String(input.tool || "").toLowerCase()
      if (!["write", "edit", "apply_patch"].includes(tool)) return
      const file =
        output.args?.filePath ||
        output.args?.path ||
        output.args?.file_path ||
        output.args?.file
      const top = topDir(file, directory)
      if (!top) return
      const nested = path.join(directory, top, "AGENTS.md")
      if (!fs.existsSync(nested)) {
        throw new Error(
          `agent-kit: blocked write under ${top}/ — no ${top}/AGENTS.md. Read docs/GROWTH.md and write a nested AGENTS.md first.`,
        )
      }
    },
    "tool.execute.after": async (input, output) => {
      const tool = String(input.tool || "").toLowerCase()
      if (!["write", "edit", "apply_patch"].includes(tool)) return
      const { status, text } = runCheck()
      const marker = "======== AGENT KIT STEER ========"
      const idx = text.indexOf(marker)
      if (idx >= 0 && output && typeof output === "object") {
        const steer = text.slice(idx)
        if (typeof output.output === "string") output.output += `\n${steer}`
        else output.output = steer
      }
      if (status !== 0) throw new Error(text || "agent-kit check failed")
    },
  }
}
