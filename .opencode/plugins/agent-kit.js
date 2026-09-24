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

function norm(p) {
  return String(p || "").replace(/\\/g, "/")
}

function relTo(filePath, root) {
  if (!filePath) return null
  const r = norm(root).replace(/\/$/, "")
  let rel = norm(filePath)
  if (rel.startsWith(r + "/")) rel = rel.slice(r.length + 1)
  if (rel.startsWith("/")) return null // outside the repo; not ours to police
  return rel
}

// Submodule paths from .gitmodules. Foreign repos: scaffold around, never inside.
function submodulePaths(fs, path, root) {
  const gm = path.join(root, ".gitmodules")
  if (!fs.existsSync(gm)) return []
  const out = []
  for (const line of fs.readFileSync(gm, "utf8").split("\n")) {
    const m = line.match(/^\s*path\s*=\s*(.+?)\s*$/)
    if (m) out.push(norm(m[1]).replace(/\/$/, ""))
  }
  return out
}

function submoduleFor(rel, subs) {
  if (!rel) return null
  return subs.find((s) => rel === s || rel.startsWith(s + "/")) || null
}

function topDir(rel) {
  if (!rel || !rel.includes("/")) return null
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
      const rel = relTo(file, directory)
      if (!rel) return

      // Hard boundary: never write inside a submodule. No bypass, no "add
      // AGENTS.md first" hint — the fix is to work in the superproject.
      const sub = submoduleFor(rel, submodulePaths(fs, path, directory))
      if (sub) {
        throw new Error(
          `agent-kit: blocked write to ${rel} — '${sub}/' is a git submodule (a foreign repository). ` +
            `Never edit or scaffold inside it. Instead: describe it in docs/submodules/, ` +
            `wrap it at the boundary in your own code, bump the pinned commit, or file the change upstream. ` +
            `Read docs/SUBMODULES.md.`,
        )
      }

      const top = topDir(rel)
      if (!top) return
      const chain = path.join(directory, "tooling/agent-kit/agents-chain.py")
      if (!fs.existsSync(chain)) {
        throw new Error(
          `agent-kit: missing tooling/agent-kit/agents-chain.py — re-run repo-scaffold init. Refusing write under ${top}/.`,
        )
      }
      const env = { ...process.env }
      delete env.AGENT_KIT_CHAIN_TEST
      delete env.AGENT_KIT_CROWD_THRESHOLD
      delete env.AGENT_KIT_SKIP
      const r = spawnSync("python3", [chain, "--guard", rel, directory], {
        encoding: "utf8",
        cwd: directory,
        env,
      })
      if ((r.status ?? 1) !== 0) {
        throw new Error(
          `${r.stderr || ""}${r.stdout || ""}`.trim() ||
            `agent-kit: chain guard failed for ${rel}. Read docs/GROWTH.md.`,
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
