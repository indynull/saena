# Projects

The store is one home desk: `$XDG_DATA_HOME/saena/desk`. Every
`saena` on the machine opens that tree. Claims, tickets, deeds,
and memory stay one exclusive graph.

A checkout joins by name. `saena init` writes `.saena/project`
(one line, the slug) and registers that name on the home desk.
`complete` snapshots the folder that holds the project file.
Ready work and `plan list` show that name's inbox.

```sh
cd ~/src/widget
saena init --project widget
saena status
```

`status` prints the desk path and `project widget`. Two
checkouts of `widget` contend on the same work ids. That is
the point of one tree.

Cards stay beside the home desk. `pin` scopes memory per
sitting.
