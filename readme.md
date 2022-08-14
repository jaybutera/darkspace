# Project Layout

Darkspace is a virtual space where you can facilitate encrypted voice communication based on locality in the space.

## Visual and Game Components

There are some minimalish game engines in rust like
[bevy](https://bevyengine.org/learn/book/getting-started/ecs/). But this still
seems like too much. All we need is to render sprites in a loop and move based
on keyboard input.

[Vulkano](https://lib.rs/crates/vulkano) and [glib](https://lib.rs/crates/glib)
graphics libraries are popular but I haven't looked deeply.

## Multiplayer

This can either be server based and self-hosted for security, or else
completely decentralized in a p2p gossip network. The latter will be slow and
there will be lots of timing incongruencies between clients, but the demands of
the vspace are not very high and some latency is fine.

p2p also means people can cheat and teleport etc. This can be a feature
not a bug.

## Privacy/Security

Both voip calls and position info should be e2e encrypted. Since info about
location can also reveal who is talking with who.

Each user will have a local public/priv key which (pub) is shared when
establishing a voice connection.

## Private Voice Calls

Two users can start a call by establishing a shared secret. Audio is then
streamed p2p.

When a third person wants to join the conversation, all 3 people need to
establish a new shared secret. When one person leaves, the remaining will
establish a new shared secret. This can be done using an iterative
diffie-hellman key exchange. A slower implementation would just encrypt for
every person's public key.

## Public Voice Calls

There are also public areas where voice communication is unencrypted.
