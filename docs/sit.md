# What a sitting is

You sit with one tree. The host admits agents. An agent claims
a ready work node, does the job, and completes. Producing work
names a **deed** — frozen bytes plus a trail. The next sitting
opens that id.

Four kinds of node:

- **Work** — who does what now. Only work is claimable.
- **Deed** — the named product. Write-once.
- **Ticket** — a plan that outlives the sitting.
- **Atom** — an extracted claim (`Remember:` / `Prefer:`).

Ready is a walk. Claim is compare-and-swap on generation. Two
actors cannot hold the same work node.

`saena sit` dumps the four faces. `saena status` is the glance:
desk path, project name, ready work, tip deed, last memory.
