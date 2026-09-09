let meta = #{
    name: "close-four",
    description: "Prove the desk sitting through the shipped saena command",
    when_to_use: "After a desk change, run the live sitting and just check",
    phases: [
        #{ title: "Prove", detail: "just check and a sitting through saena" },
    ],
};

let prove_schema = #{
    "type": "object",
    "required": ["ok", "command", "notes"],
    "properties": #{
        "ok": #{ "type": "boolean" },
        "command": #{ "type": "string" },
        "notes": #{ "type": "string" },
    },
};

let root = if args == () { () } else { args.checkout };
if root == () {
    pause("verification", "Pass args.checkout — absolute path of the saena desk repo.");
}

phase("Prove");
let pp = "";
pp += "Prove the saena desk in " + json_encode(root) + ".\n";
pp += "Run `just check`. Then a live sitting through target/debug/saena: work, claim exclusive, complete names a deed, ticket stays open, remember refuses a dump, search hits search.milli, verify names a cycle, quote then patch trail, pin scopes remember.\n";
pp += "Return {ok, command, notes} with the exact commands you ran.";
let proved = agent(pp, #{
    label: "prove-sitting",
    capability_mode: "all",
    output_schema: prove_schema,
});
if proved == () || proved.success == false || proved.output == () || proved.output.ok != true {
    let notes = "";
    if proved != () && proved.output != () && proved.output.notes != () {
        notes = proved.output.notes;
    }
    complete(#{ summary: "Sitting did not prove", notes: notes });
}
complete(#{
    summary: "Desk sitting proved",
    command: proved.output.command,
});
