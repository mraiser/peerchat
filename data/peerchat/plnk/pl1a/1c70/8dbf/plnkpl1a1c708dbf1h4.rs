let peer = nn_session.try_get_string("username").unwrap_or_default();
if !valid(&peer) { panic!("peerchat receive is for peer connections only"); }
if id.is_empty() || id.len() > 64 || text.is_empty() || text.len() > MAX_TEXT { panic!("bad message"); }

let dup = messages(&peer).iter().any(|m| !m.get_boolean("me") && m.get_string("id") == id);
if !dup {
  let now = now_ms();
  let mut row = DataObject::new();
  row.put_string("id", &id);
  row.put_boolean("me", false);
  row.put_string("text", &text);
  row.put_int("time", now);
  row.put_int("sent_time", time);
  append(&peer, &row);
  touch(&peer, &text, now, 1);
  nudge(&peer);
}
"OK".to_string()