# Saena

Saena is a desk. A person sits with one tree. Unbounded nested
agents claim on the same live graph. A sitting ends by naming a
**deed**. The next sitting opens that id.

## One tree

Every id is a node. Kind and status are fields. Parent, blocks,
cites, sources, and “this work named this deed” are edges. Ready
is a walk. `claim` is compare-and-swap on generation. Search is
milli, then the host panel (default Borda, then diversify).
`verify` walks work links. `pin` scopes remember and search.

| Kind | Job | Lifetime |
| --- | --- | --- |
| **Work** | Who does what now. Only work is claimable. | this sitting, plus restore |
| **Deed** | Frozen named product. Write-once bytes, evidence, trail, `current`, `leave`, timestamp. Kinds: file, set, quote, patch, mailDraft, clip, page, form, table, procedure, event. | frozen after create |
| **Ticket** | Durable plan: parent, blocked-by, ready, note, append, related, agenda. | outlives the sitting |
| **Atom** | Extracted claims. `Remember:` / `Prefer:` are instant. | extracted, host-owned |

The host mints a live work node from a ready ticket. Agents claim
that work. Completing work may name a deed and may cite a ticket.
The ticket stays until the person closes it.

Summary is the only open text on a work node. Kind, bytes, and
trail live on the deed. Memory atoms are extracted claims. Human
cards are `USER.md` and `MEMORY.md` beside the desk.

The tree lives in one Lightning Memory-Mapped Database. Bytes sit
beside it, content-addressed. The store is the home desk,
`$XDG_DATA_HOME/saena/desk` (`SAENA_DESK` overrides). A checkout
writes `.saena/project` with a name. That name is the inbox.
`complete` snapshots that folder.

Work and tickets carry the project name. Ready and list show the
checkout you sat in. File a ticket in the project that must
implement it (`plan --project beta`). The host mints work there.
The requester's work **depends** on that work. `wait` sleeps until
the node is terminal. The deed is on the completed work. A
cancelled node is a bounce, not a product.

## The desk

`saena sit` is the terminal face. A child is another worker on
the same desk. The `saena` command and the Model Context Protocol
share the sitting verbs. `saena surface` lists them.

## How a sitting runs

1. Open a desk, or open a deed id from a prior sitting.
2. The host admits agents. Spawn inserts child work nodes under
   the parent node.
3. An admitted actor **claims** a ready work node. Two actors
   cannot hold the same node. The host is the sole mutator.
4. `Remember:` / `Prefer:` hang an atom on the tree. The host
   commits atoms. Tool dumps are refused.
5. The actor **completes** the node. Producing work creates a
   deed and names that id. Empty or failed work leaves no deed.
6. The sitting names the tip deed. The next sitting opens that
   id. `evidence` checks the product and its sources. `trail`
   walks `sources`. Tickets and atoms remain.
