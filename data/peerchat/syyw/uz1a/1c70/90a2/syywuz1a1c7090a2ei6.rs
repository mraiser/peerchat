let text = text.trim().to_string();
if !valid(&peer) { panic!("not a peer id"); }
if text.is_empty() { panic!("empty message"); }
if text.len() > MAX_TEXT { panic!("message too long"); }

let now = time();
let mut row = DataObject::new();
row.put_string("id", &new_id());
row.put_boolean("me", true);
row.put_string("text", &text);
row.put_int("time", now);
append(&peer, &row);
touch(&peer, &text, now, 0);

// Delivery never blocks the page: it runs in the background and an
// event tells the page when the peer has acknowledged.
let p = peer.clone();
std::thread::spawn(move || deliver(&p));

row.put_boolean("sent", false);
row