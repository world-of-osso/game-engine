# Lore Knowledge Graph

A server-side relational graph tracking all world lore — NPCs, factions, locations, items, events, and their relationships. Used by AI-driven NPCs to act consistently and generate contextually valid quests without contradicting established lore.

## Why a Graph

Lore is fundamentally relational. Flat tables can't answer "who are the enemies of my faction's allies?" without recursive joins. A graph makes these queries natural: `(Thrall, leads, Horde)` is a triple; traversal is direct.

## Graph Schema

**Node types**: `npc`, `faction`, `location`, `item`, `event`

**Edge types**: `member_of`, `hostile_to`, `allied_with`, `located_in`, `knows`, `trusts`/`distrusts`, `witnessed`, `caused_by`, `requires`

Key properties: NPCs have `alive: bool`; events have timestamps; factions have alignment and territory.

## Consistency Rules

Enforced before any mutation (NPC action, quest creation, event resolution):

1. Dead NPCs don't act (`npc.alive = false` → reject edge creation)
2. Location coherence — NPCs only interact with entities in the same/connected location
3. Faction alignment — NPC can't create quests helping a `hostile_to` faction unless `defector`/`double_agent`
4. Event ordering — no NPC can reference a future event
5. No contradictory edges — `allied_with` and `hostile_to` can't coexist between same factions
6. `knows` is symmetric; `trusts`/`distrusts` is directional

## NPC-Generated Quests

NPCs formulate quests based on personality + graph state. Stored in `npc_quests` table (SQLite for complex queries):

| Field | Purpose |
|-------|---------|
| `creator_npc` | NPC who generated it |
| `objective_type` | fetch / escort / eliminate / deliver / investigate / gather_info |
| `target_nodes` | graph node IDs involved |
| `expires_at` | ephemeral — expires when world state changes |
| `graph_snapshot` | relevant subgraph at creation (conflict detection) |

**Generation flow**: personality + graph state → need → validate consistency rules → insert to `npc_quests` → offer to player → on resolution, mutate graph (e.g. `eliminate` → `npc.alive = false`).

**Conflict detection**: no two active quests with contradictory objectives on the same nodes; rate limit per NPC.

## Storage

- **redb tables** — node/edge data (same store as game-server persistence)
- **SQLite** — `npc_quests` table (expiration, status tracking, complex queries)

## Authority

The knowledge graph lives **exclusively on game-server**. Clients never have a copy. NPC dialogue and quest text are sent as pre-rendered strings; the server resolves all graph queries server-side.

## Open Questions

- Graph query latency at scale — may need spatial partitioning (subgraph per zone)
- Interaction between NPC-created quests and designer-authored quest chains

## Sources

- [lore-knowledge-graph.md](../lore-knowledge-graph.md) — full schema, consistency rules, quest generation design

## See Also

- [[networking]] — server-side authority; quest/NPC data flows to client over lightyear
