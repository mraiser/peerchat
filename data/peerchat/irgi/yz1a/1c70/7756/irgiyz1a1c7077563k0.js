var me = this;
var ME = $('#'+me.UUID)[0];

// All state lives here; the DOM only renders it.
var S = { chats: [], peer: null, msgs: [] };

function pad(n){ return n < 10 ? '0' + n : '' + n; }
function clock(t){ var d = new Date(t); return pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function day(t){
  var d = new Date(t), now = new Date();
  if (d.toDateString() == now.toDateString()) return 'Today';
  var y = new Date(now.getTime() - 86400000);
  if (d.toDateString() == y.toDateString()) return 'Yesterday';
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}
function stamp(t){ return !t ? '' : day(t) == 'Today' ? clock(t) : day(t); }
function current(){ for (var i in S.chats) if (S.chats[i].id == S.peer) return S.chats[i]; return null; }

// ---- chat list ----
function renderList(){
  var list = $(ME).find('.pc-list').empty();
  var chats = S.chats.slice().sort(function(a, b){
    return (b.time - a.time) || (b.connected - a.connected) || a.name.localeCompare(b.name);
  });
  $(ME).find('.pc-none').attr('hidden', chats.length ? true : null);
  chats.forEach(function(c){
    var row = $('<div class="pc-chat">').toggleClass('on', c.id == S.peer);
    row.append($('<span class="pc-dot">').toggleClass('on', !!c.connected));
    row.append($('<span class="pc-chat-name">').text(c.name));
    row.append($('<span class="pc-chat-time">').text(stamp(c.time)));
    row.append($('<span class="pc-chat-last">').text(c.last || ''));
    if (c.unread > 0) row.append($('<span class="pc-badge">').text(c.unread > 99 ? '99+' : c.unread));
    row.click(function(){ open(c.id); });
    list.append(row);
  });
}

function renderHead(){
  var c = current();
  if (!c) return;
  $(ME).find('.pc-head .pc-dot').toggleClass('on', !!c.connected);
  $(ME).find('.pc-name').text(c.name);
  $(ME).find('.pc-state').text(c.connected ? 'online' : 'offline - messages wait until they connect');
}

function loadChats(){
  send_chats(function(r){
    if (!r || r.status != 'ok' || !Array.isArray(r.data)) return;
    S.chats = r.data;
    var c = current();
    // an incoming message for the open thread (seen here without the event): read it
    if (c && c.unread > 0) { c.unread = 0; loadThread(); }
    renderList();
    renderHead();
  });
}

// ---- thread ----
function renderThread(){
  var box = $(ME).find('.pc-thread');
  var el = box[0];
  var atEnd = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
  box.empty();
  if (!S.msgs.length) box.append($('<div class="pc-hello">').text('No messages yet. Say hello.'));
  var last = '';
  S.msgs.forEach(function(m){
    var d = day(m.time);
    if (d != last) { box.append($('<div class="pc-day">').text(d)); last = d; }
    var b = $('<div class="pc-msg">').toggleClass('me', !!m.me).toggleClass('pending', !!m.me && !m.sent);
    b.append($('<div class="pc-text">').text(m.text));
    b.append($('<div class="pc-meta">').text(clock(m.time) + (m.me ? (m.sent ? ' ✓' : ' · waiting') : '')));
    box.append(b);
  });
  if (atEnd || S.jump) { el.scrollTop = el.scrollHeight; S.jump = false; }
}

function loadThread(){
  var p = S.peer;
  if (!p) return;
  send_history(p, function(r){
    if (S.peer != p || !r || r.status != 'ok' || !Array.isArray(r.data)) return;
    S.msgs = r.data;
    renderThread();
  });
}

function open(id){
  S.peer = id;
  S.msgs = [];
  S.jump = true;
  $(ME).find('.pc').addClass('open');
  $(ME).find('.pc-blank').attr('hidden', true);
  $(ME).find('.pc-head, .pc-thread, .pc-form').removeAttr('hidden');
  var c = current();
  if (c) c.unread = 0;
  renderList();
  renderHead();
  renderThread();
  loadThread();
  $(ME).find('.pc-input').focus();
}

$(ME).find('.pc-back').click(function(){
  S.peer = null;
  $(ME).find('.pc').removeClass('open');
  renderList();
});

// ---- composer: Enter sends, Shift+Enter is a new line ----
var input = $(ME).find('.pc-input');
function fit(){
  input[0].style.height = 'auto';
  input[0].style.height = Math.min(input[0].scrollHeight + 2, 140) + 'px';
  $(ME).find('.pc-send').attr('disabled', input.val().trim() ? null : true);
}
input.on('input', fit);
input.on('keydown', function(e){
  if (e.key == 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); $(ME).find('.pc-form').submit(); }
});
$(ME).find('.pc-form').on('submit', function(e){
  e.preventDefault();
  var text = input.val().trim(), p = S.peer;
  if (!text || !p) return;
  input.val('');
  fit();
  send_send(p, text, function(r){
    var m = r && r.status == 'ok' && r.data && r.data.id ? r.data : null;
    if (!m) {
      // not stored: give the text back rather than lose it
      if (S.peer == p && !input.val()) { input.val(text); fit(); }
      alert('That message could not be saved: ' + ((r && (r.msg || (r.data && r.data.msg))) || 'unknown error'));
      return;
    }
    if (S.peer == p) { S.msgs.push(m); S.jump = true; renderThread(); }
    loadChats();
  });
});

// ---- live updates: events when the websocket is up, a slow poll regardless ----
function subscribe(){
  if (typeof SOCK == 'undefined' || !SOCK || SOCK.readyState != 1) return setTimeout(subscribe, 500);
  subscribe_event('peerchat', 'MESSAGE', function(d){
    if (d && d.peer == S.peer) loadThread();
    loadChats();
  });
  subscribe_event('peer', 'UPDATE', loadChats);
}

me.ready = function(){
  loadChats();
  subscribe();
  setInterval(loadChats, 15000);
};
