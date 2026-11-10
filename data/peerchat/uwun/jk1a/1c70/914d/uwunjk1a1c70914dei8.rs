if !valid(&peer) { panic!("not a peer id"); }
let all = messages(&peer);
let skip = all.len().saturating_sub(500);
let mut out = DataArray::new();
for m in all.into_iter().skip(skip) { out.push_object(m); }
touch(&peer, "", 0, -1);
out