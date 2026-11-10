let st = state();
let mut out = DataArray::new();
for p in crate::api::peer::peer::peers().objects() {
  let p = p.object();
  let id = p.get_string("id");
  let mut o = DataObject::new();
  o.put_string("id", &id);
  o.put_string("name", &p.try_get_string("name").unwrap_or(id.clone()));
  o.put_boolean("connected", p.try_get_boolean("connected").unwrap_or(false));
  let e = if st.has(&id) { st.get_object(&id) } else { DataObject::new() };
  o.put_int("unread", e.try_get_int("unread").unwrap_or(0));
  o.put_string("last", &e.try_get_string("last").unwrap_or_default());
  o.put_int("time", e.try_get_int("time").unwrap_or(0));
  out.push_object(o);
}
out