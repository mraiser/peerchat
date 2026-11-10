if !valid(&id) { return "ERROR: not a peer id".to_string(); }
deliver(&id);
"OK".to_string()
}

// ---- shared by send / receive / history / chats ----

pub const MAX_TEXT: usize = 16000;
static FILES: Mutex<()> = Mutex::new(());
static BUSY: Mutex<Vec<String>> = Mutex::new(Vec::new());
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A peer id is a uuid; this is also what keeps it safe as a file name.
pub fn valid(peer: &str) -> bool {
  peer.len() == 36 && peer.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub fn new_id() -> String {
  format!("{:x}-{:x}-{:x}", time(), std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst))
}

fn path(peer: &str) -> PathBuf {
  let root = DataStore::new().root.parent().unwrap().to_path_buf();
  root.join("runtime").join("peerchat").join("chats").join(format!("{}.jsonl", peer))
}

/// One conversation = one append-only file of rows: a message {id, me, text, time}
/// or a delivery receipt {ack: <message id>, time}.
pub fn append(peer: &str, row: &DataObject) {
  let _g = FILES.lock().unwrap_or_else(|e| e.into_inner());
  let p = path(peer);
  let _ = std::fs::create_dir_all(p.parent().unwrap());
  let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&p).expect("cannot open chat file");
  writeln!(f, "{}", row.to_string().replace('\n', " ")).expect("cannot write chat file");
}

/// The conversation folded: messages in arrival order, each with `sent`
/// (incoming: always true; mine: true once the peer acknowledged it).
pub fn messages(peer: &str) -> Vec<DataObject> {
  let mut out: Vec<DataObject> = Vec::new();
  let mut acked: HashSet<String> = HashSet::new();
  if let Ok(s) = std::fs::read_to_string(path(peer)) {
    for line in s.lines() {
      if !line.starts_with('{') { continue; }
      if let Ok(r) = DataObject::try_from_string(line) {
        if let Ok(a) = r.try_get_string("ack") { acked.insert(a); }
        else if r.has("id") && r.has("text") { out.push(r); }
      }
    }
  }
  for m in out.iter_mut() {
    let me = m.try_get_boolean("me").unwrap_or(false);
    let sent = !me || acked.contains(&m.get_string("id"));
    m.put_boolean("me", me);
    m.put_boolean("sent", sent);
  }
  out
}

/// Unread count and last message per peer: one small record in the runtime library.
pub fn state() -> DataObject {
  let store = DataStore::new();
  if store.exists("runtime", "peerchat_state") { store.get_data("runtime", "peerchat_state").get_object("data") }
  else { DataObject::new() }
}

/// unread: 1 = one more, -1 = reset to zero, 0 = unchanged. Empty text leaves `last` alone.
pub fn touch(peer: &str, text: &str, when: i64, unread: i64) {
  let _g = FILES.lock().unwrap_or_else(|e| e.into_inner());
  let mut st = state();
  let mut e = if st.has(peer) { st.get_object(peer) } else { DataObject::new() };
  let n = e.try_get_int("unread").unwrap_or(0);
  if unread < 0 && n == 0 && text.is_empty() { return; }
  e.put_int("unread", if unread < 0 { 0 } else { n + unread });
  if !text.is_empty() {
    let short: String = text.chars().take(120).collect();
    e.put_string("last", &short);
    e.put_int("time", when);
  }
  st.put_object(peer, e);
  let mut rec = DataObject::new();
  rec.put_string("id", "peerchat_state");
  rec.put_object("data", st);
  rec.put_string("username", "system");
  rec.put_int("time", time());
  rec.put_array("readers", DataArray::new());
  rec.put_array("writers", DataArray::new());
  DataStore::new().set_data("runtime", "peerchat_state", rec);
}

/// Tell open pages that a conversation changed. The event names the peer only;
/// the page re-reads through the gated history command.
pub fn nudge(peer: &str) {
  let mut d = DataObject::new();
  d.put_string("peer", peer);
  fire_event("peerchat", "MESSAGE", d);
}

/// Deliver my unacknowledged messages to the peer, oldest first, stopping at the
/// first failure so order holds. One delivery run per peer at a time; whatever is
/// left stays pending until the next send or the peer's next CONNECT.
pub fn deliver(peer: &str) {
  {
    let mut busy = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    if busy.iter().any(|p| p == peer) { return; }
    busy.push(peer.to_string());
  }
  let run = std::panic::catch_unwind(|| {
    let mut done: HashSet<String> = HashSet::new();
    loop {
      let pending: Vec<DataObject> = messages(peer).into_iter()
        .filter(|m| !m.get_boolean("sent") && !done.contains(&m.get_string("id"))).collect();
      if pending.is_empty() { break; }
      let mut failed = false;
      for m in pending {
        let mid = m.get_string("id");
        let mut p = DataObject::new();
        p.put_string("id", &mid);
        p.put_string("text", &m.get_string("text"));
        p.put_int("time", m.get_int("time"));
        let r = crate::api::peer::service::exec(peer.to_string(), "peerchat".to_string(), "receive".to_string(), p);
        let ok = r.try_get_string("status").unwrap_or_default() == "ok"
          && r.try_get_string("msg").unwrap_or_default() == "OK";
        if !ok { failed = true; break; }
        let mut ack = DataObject::new();
        ack.put_string("ack", &mid);
        ack.put_int("time", time());
        append(peer, &ack);
        done.insert(mid);
        nudge(peer);
      }
      if failed { break; }
    }
  });
  BUSY.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| p != peer);
  if run.is_err() { eprintln!("peerchat: delivery to {} failed", peer); }
}

#[allow(dead_code)]
fn _end() {