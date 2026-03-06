# Lore Knowledge Graph

A knowledge graph that tracks all world lore — NPCs, factions, locations, events, relationships — so AI-driven NPCs can act consistently and dynamically generate quests without contradicting established lore.

## Why a Graph

Lore is fundamentally relational. "Thrall leads the Horde" is a triple (Thrall, leads, Horde). Flat tables can't answer "who are the enemies of my faction's allies?" without recursive joins. A graph makes these queries natural and cheap.

## Node Types

| Type | Examples | Key Properties |
|------|----------|----------------|
| `npc` | Innkeeper Allison, Guard Marcus | name, race, class, personality, backstory, location, alive |
| `faction` | Stormwind Guard, Defias Brotherhood | name, alignment, territory |
| `location` | Goldshire, Northshire Abbey | name, zone, coords, type (town/dungeon/wild) |
| `item` | Hogger's Claw, Old Blanchy's Feed | name, origin, significance |
| `event` | The Defias Uprising, Harvest Festival | name, time, participants, resolved |

## Edge Types

| Edge | From → To | Meaning |
|------|-----------|---------|
| `member_of` | npc → faction | NPC belongs to faction |
| `hostile_to` | faction → faction | Factions are enemies |
| `allied_with` | faction → faction | Factions cooperate |
| `located_in` | npc/item → location | Physical presence |
| `knows` | npc → npc | NPCs are acquainted |
| `trusts` / `distrusts` | npc → npc | Relationship quality |
| `witnessed` | npc → event | NPC saw or participated |
| `caused_by` | event → npc/faction | Who triggered an event |
| `requires` | item → item/event | Dependency chain |

## Consistency Rules

The graph enforces invariants before any mutation (NPC action, quest creation, event resolution):

1. **Dead NPCs don't act.** If `npc.alive = false`, reject any edge creation from that node.
2. **Location coherence.** An NPC can only interact with entities in the same location (or connected locations). No telepathy across zones.
3. **Faction alignment.** An NPC cannot create a quest that helps a faction `hostile_to` their own faction, unless their personality explicitly has a `defector` or `double_agent` trait.
4. **Event ordering.** Events have timestamps. No NPC can reference an event that hasn't happened yet.
5. **No contradictory edges.** `allied_with` and `hostile_to` between the same two factions is invalid. One must be removed before the other is added.
6. **Relationship symmetry.** `knows` is bidirectional. If A knows B, B knows A. `trusts`/`distrusts` is directional (A can trust B without B trusting A).

## NPC Backstory & Personality

Each NPC has a pre-generated backstory stored as graph edges + a personality profile:

```
npc: Guard Marcus
  backstory edges:
    - (Marcus, member_of, Stormwind Guard)
    - (Marcus, witnessed, Defias Raid on Goldshire)
    - (Marcus, distrusts, Defias Brotherhood)
    - (Marcus, located_in, Goldshire)
  personality:
    disposition: protective
    traits: [vigilant, distrustful_of_strangers, loyal]
    motivation: keep_goldshire_safe
```

The personality constrains what quests the NPC can generate. A `loyal` guard won't ask players to steal. A `distrustful` NPC won't send players to negotiate peace.

## NPC-Created Quests

When an AI-driven NPC decides to formulate a request, it becomes a quest. These are stored in a dedicated table, separate from designer-authored quests.

### `npc_quests` Table

| Column | Type | Description |
|--------|------|-------------|
| `id` | uuid | Unique quest ID |
| `creator_npc` | npc_id | The NPC who generated this quest |
| `status` | enum | `pending`, `accepted`, `in_progress`, `completed`, `expired`, `failed` |
| `motivation` | text | Why the NPC wants this (derived from personality + graph state) |
| `objective_type` | enum | `fetch`, `escort`, `eliminate`, `deliver`, `investigate`, `gather_info` |
| `target_nodes` | npc_id[] / item_id[] / location_id[] | Graph nodes involved in the objective |
| `reward_type` | enum | `gold`, `item`, `reputation`, `information`, `favor` |
| `expires_at` | timestamp | NPC quests are ephemeral — they expire if the world state changes |
| `graph_snapshot` | jsonb | Relevant subgraph at creation time (for conflict detection) |
| `created_at` | timestamp | When the NPC formulated the request |

### Quest Generation Flow

1. **Trigger.** NPC's personality + current graph state produces a need. (Guard Marcus sees Defias activity near Goldshire → motivation: "eliminate threat")
2. **Validate against graph.** Check consistency rules. Is the target alive? Is the location reachable? Does this contradict faction alignment?
3. **Insert into `npc_quests`.** Store the quest with a graph snapshot of relevant nodes/edges.
4. **Offer to players.** NPC presents the quest in-character, drawing from backstory and personality.
5. **Resolve.** On completion/failure/expiration, update the graph. (Defias camp destroyed → add event node, update faction edges.)

### Conflict Detection

Before creating a quest, check:

- No other active `npc_quest` targets the same nodes with a contradictory objective (two NPCs asking to both protect and kill the same target).
- The quest doesn't violate any consistency rule from the graph.
- The NPC hasn't exceeded a quest generation rate limit (prevents spamming).

### Graph Mutation on Resolution

Quest completion can mutate the graph:

- `eliminate` quest completed → target NPC marked `alive = false`
- `deliver` quest completed → item `located_in` edge updated
- `gather_info` quest completed → new `knows` edges created
- Failed/expired quests may also mutate (NPC loses trust in players, situation worsens)

## Storage

The knowledge graph can be stored as:

- **redb tables** for the node/edge data (same as game-server persistence)
- **SQLite** for the `npc_quests` table (complex queries, expiration, status tracking)

Both are local, file-based, no external dependencies.

## Authority

The knowledge graph lives exclusively on the **game-server**. The client never has a copy — not even read-only.

- NPC dialogue, quest text, and behavioral cues are sent to the client as pre-rendered strings/packets.
- The client requests quest info via the network protocol; the server resolves graph queries and returns results.
- This keeps the graph a server-side secret — players can't datamine NPC relationships or upcoming quest triggers.

## Open Questions

- How large can the graph get before query latency affects NPC decision-making? May need spatial partitioning (only query subgraph for current zone).
- How do NPC-created quests interact with designer-authored quest chains? Priority? Mutual exclusion?
