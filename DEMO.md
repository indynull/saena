# Demo: a todo app, three agents at once

```sh
mkdir -p ~/src/todo && cd ~/src/todo
```

In that folder:

```sh
saena init
```

Open Grok there. Paste in order. Do not type saena verbs.

## Paste 1

```
Build a local todo web app here: add, check off, delete, persist
across reload. No framework.

Sit. status. Remember that todos persist in localStorage on one
HTML page with no build step.

Plan store, ui, persist, and app. Block app on the other three.
Claim a parent, spawn three children (store, ui, persist). Launch
three agents in parallel in this directory. Each claims one ready
child and implements only that slice:

- store.js — items, add/toggle/remove, ids
- ui.js — list, input, checkbox, delete
- persist.js — localStorage load/save

When a slice is done, complete the node. Do not pass file paths.
When all three are done, write index.html that loads them, complete
the parent, and print status.
```

## Paste 2 — new Grok, same folder

```
Sit. status. Open the tip deed. Do not reread the last chat.
Add a "clear done" control. Complete. Print the new tip.
```

Open `index.html` after paste 1.
