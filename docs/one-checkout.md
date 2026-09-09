# One checkout

You are in one folder. You want a named product.

```sh
saena work "name the note"
saena work claim w-… --as alice
printf 'a named note\n' > note.txt
saena work complete w-… --file note.txt
saena deed get d-…
```

`complete` without `--file` snapshots files that changed since
claim. Empty work leaves no deed.

The next sitting is `saena deed get d-…`. Same id, same bytes.

Grok paste: sit, `status`, claim ready work or `work` then
claim, do the job, `complete`. Do not ask for the desk path.
