# peerchat

Instant messaging between [Newbound](https://github.com/mraiser/newbound)
peers. A chat list on the left, a thread on the right, and nothing in
between but the peer network you already have: messages travel over
Newbound's encrypted peer connections, straight from your instance to
the other one. There is no server, no account and no relay of its own.

## Install

Both ends of a conversation need peerchat installed.

1. Open the **Dev** app → shelf → **git import** and import
   `https://github.com/mraiser/peerchat` (or call `dev.github.import`
   with that URL). The platform clones the repo into
   `repositories/peerchat` and links `data/peerchat` into place.
2. Restart Newbound once. The first install adds the `peerchat`
   hot-reload crate to the build; later code updates hot-load without a
   restart.
3. Open `/peerchat/index.html` (activate the **peerchat** app in the app
   list if it is not there, or add `peerchat` to the `apps=` line of
   `config.properties`).

To update: `dev.github.update` with library `peerchat`. To uninstall:
`dev.github.remove`.

## Use

- The left pane lists every peer your instance knows, with an online
  dot, the last message, its time and an unread badge.
- Pick a peer and type. **Enter** sends, **Shift+Enter** adds a line.
- A message you send shows a "waiting" mark until the other instance
  acknowledges it. If the peer is offline or does not have peerchat yet,
  the message stays pending and is delivered when that peer next
  connects (or when you next send to it).
- On a narrow screen the app shows one pane at a time.

## Who can message you

The `receive` command is open to peers in the `admin` or `peerchat`
security group. To let a non-admin peer chat with you, add `peerchat`
to that peer's groups in the Security app. The sender of a message is
always the authenticated peer connection it arrived on, never a field
in the message.

## Where your messages live

Nothing conversational is stored in this repository or in the library.

- Each conversation is an append-only file,
  `runtime/peerchat/chats/<peer-uuid>.jsonl`: message rows
  `{id, me, text, time}` and receipt rows `{ack, time}`.
- Unread counts and last-message previews are one record,
  `peerchat_state`, in the runtime library.

Messages are plain text on disk on both instances; back up or delete
the `chats` directory as you would any other private file.

## How it works

One control, `peerchat.peerchat`, with five commands:

| Command | What it does |
| --- | --- |
| `chats` | The chat list: every known peer with last message, time and unread count. |
| `history` | One conversation, oldest first (the latest 500 messages); marks it read. |
| `send` | Stores the message as pending, returns at once, delivers in the background. |
| `receive` | The door other peers call. Creates the conversation on first contact and stores each message id once, so a retried delivery is acknowledged, not duplicated. |
| `flush` | Delivers pending messages to one peer, oldest first, stopping at the first failure. Fired by the peer `CONNECT` event and by `send`. |

Delivery is a `peer.service.exec` call to the other instance's
`receive`; its `OK` is recorded as the receipt. Both sides fire a
`peerchat` `MESSAGE` event so an open page updates without polling.

## Limits

- Text only, one-to-one. No groups, attachments, edits or deletes.
- Message text is 1 to 16000 bytes.
- Delivery waits on the other instance's answer without a timeout, so a
  peer that hangs holds up later messages to that same peer until it
  answers or reconnects.
- A message sent at the exact moment a delivery run to the same peer is
  finishing can stay pending until the next send or reconnect.
