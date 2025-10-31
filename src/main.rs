fn main() {
    use std::io::{self, Write};
    use kvcrdt::engine::kv::KvStore;

    let store = KvStore::new();
    println!("kvcrdt in-memory KV store. Type 'help' for commands.");

    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            eprintln!("error: failed to read input");
            continue;
        }
        let line = line.trim();
        if line.is_empty() { continue; }

        let mut parts = line.splitn(3, ' ');
        let cmd = parts.next().unwrap().to_lowercase();

        match cmd.as_str() {
            "put" => {
                let key = match parts.next() { Some(k) => k.to_string(), None => { eprintln!("usage: put <key> <value>"); continue; } };
                let value = match parts.next() { Some(v) => v.to_string(), None => { eprintln!("usage: put <key> <value>"); continue; } };
                store.put(key, value);
                println!("OK");
            }
            "get" => {
                let key = match parts.next() { Some(k) => k, None => { eprintln!("usage: get <key>"); continue; } };
                match store.get(key) {
                    Some(v) => println!("{}", v),
                    None => println!("(nil)"),
                }
            }
            "del" | "delete" => {
                let key = match parts.next() { Some(k) => k, None => { eprintln!("usage: del <key>"); continue; } };
                let removed = store.delete(key);
                println!("{}", if removed { 1 } else { 0 });
            }
            "help" => {
                println!("Commands:\n  put <key> <value>\n  get <key>\n  del <key>\n  exit | quit | Ctrl+D");
            }
            "exit" | "quit" => {
                println!("bye");
                break;
            }
            _ => {
                eprintln!("unknown command: {}", cmd);
            }
        }
    }
}
