# Terrain Chess — Overworld Design

Replaces the floor-based run in [game-design.md §4](game-design.md) once implemented.

---

## 1. Premise & story frame
The realm of Oakhaven is fractured. Two rival powers—the holy kingdom of the Ashen Sun led by Lord Caelen, and the undead court of the Hollow Crown led by the Lich-King Malakor—have arrived to claim its lands. 

To conquer Oakhaven, the lords must scour the countryside, rally scattered garrisons to their banners, and build an unstoppable army. It is a race against time and each other: whoever musters a complete force first will crush their rival and rule the realm.

## 2. Run structure
- **Start:** The run begins on a generated 2D overworld map. Two heroes (the player and the AI rival) spawn in their own starting regions with a basic army: 1 King and 3 Pawns.
- **Day Loop:** The game proceeds in "Days" (turns). Each day, heroes receive a pool of movement points. They take turns moving across the map, exploring, collecting items, and attacking camps to recruit new pieces. Battles take place on a separate chess board.
- **End:** The run ends in victory if the player defeats the rival hero in battle (checkmating their King). The run ends in defeat if the player's King is checkmated by any enemy or the rival hero. Win by being the last hero standing.

## 3. The map
- **Size and Generation:** A large grid (e.g., 64×64 or 128×128 tiles) generated from a single seed using noise, with distinct starting regions for each hero. Camps and pickups are distributed fairly so each start region has equal value.
- **Regions:** The map is divided into zones (plains, forests, rugged hills) connecting the two start areas.
- **Terrain Mapping (Map vs Board):** The tile where a battle occurs dictates the biome and rough layout of the 8×8 battle board. An overworld forest tile generates a board with many trees; a hills tile generates high cliffs.
- **Movement & Roads:** Moving costs 1 movement point per flat tile. Forests and hills cost 2. Roads connect key camps and cost only 0.5 points to travel.
- **Fog of War:** Yes. The map is initially obscured. Heroes have a sight radius that reveals tiles and camps permanently as they move.

## 4. Camps
Camps are scattered nodes guarded by neutral or local forces. A camp fields its captain (King), its boss and optional pawn guards; the battle is won by checkmate as usual. Defeating a camp adds its boss to your permanent army. Camps do not respawn once cleared.

| Camp Type | Boss Reward | Guards | Rarity |
|---|---|---|---|
| **Village** | Pawn | Captain, 2 Pawns | Common |
| **Knight Camp** | Knight | Captain, 1 Knight | Uncommon |
| **Bishop Camp** | Bishop | Captain, 1 Bishop | Uncommon |
| **Fortress** | Rook | Captain, 1 Rook, 1 Pawn | Rare |
| **Citadel** | Queen | Captain, 1 Queen, 2 Pawns | Epic (1 per map) |

*Note: Guards scale with day count. A complete team is 16 pieces (K, Q, 2R, 2B, 2N, 8P).*

### Map objects
Scattered nodes and pickups that do not trigger battles:
- **Chests (closed/open):** Grants an item or spell card added to your permanent deck/inventory.
- **Shrines:** Grants a permanent relic or passive blessing to your army.
- **Signposts:** Reveals surrounding fog of war and highlights nearby points of interest.

## 5. Battles
- **Initiation:** Moving onto a camp or enemy hero tile starts a battle.
- **Camp Guards' Faction:** Neutral camps field the faction opposite to the attacking hero on the battle board (Ashen Sun pieces defend if the attacker is Hollow Crown, and Hollow Crown pieces defend if the attacker is Ashen Sun). In a later art pass, these will be tinted with a "neutral" palette.
- **Deployment Step:** Before the first move, players place their army on the 8×8 board within their own back two ranks (the deployment zone). The deployment zone is 16 squares total, so at most 16 pieces can be fielded.
- **Mapping Army to Board:** You deploy pieces from your roster. The King must always be deployed. If you have fewer than 16 pieces, your deployment zone will have gaps. If you have more than 16 pieces, you choose which 16 to field; any pieces left out remain safely in your roster for future battles.
- **Casualties & Carry-over:** Pieces captured during a battle are lost permanently from your army. Piece enhancements (upgrades) carry over between battles.
- **Retreat:** A player can choose to retreat (resign) during their turn. The hero survives, but returns to the tile they entered from with 0 movement points left for the remainder of that day. Any pieces already captured in that battle remain lost. If retreating from a camp, the camp keeps its remaining surviving guards (partially cleared). The battle ends immediately.

## 6. Cards & rewards
- **Permanent Deck:** Instead of drafting temporary rewards per match, post-battle drafts provide permanent cards (spells, piece enhancements, relics) added to your run's deck/inventory.
- **Deck Limits:** The permanent deck can grow indefinitely. 
- **SpellHand Mechanics:** Before each match, the player builds a 15-card `SpellHand` deck from their permanent collection. Only these 15 cards are brought into the battle.

## 7. AI rival hero
The AI hero roams the overworld simultaneously, following a simple goal-driven behavior:
1. **Explore & Expand:** Pick the best reachable camp based on a score combining value (boss type), distance (movement points required), and risk (AI's current army strength vs guard scaling).
2. **Hunt:** If the AI's army value is significantly higher than the player's known army, it paths toward the player to initiate a final battle.
3. **Camp Battles (Auto-resolve):** The AI rival's camp fights are auto-resolved off-screen rather than played as chess matches. Resolution is calculated deterministically from the AI's army value versus the camp's guard value with seeded randomness. Casualties for the AI scale inversely with how decisive the victory is (closer fights inflict more piece losses). Only battles involving the player are played on the chess board.

## 8. Hero vs hero
- **Initiating:** Moving onto the tile occupied by the rival hero initiates the decisive battle.
- **Defending:** The defending hero gets to deploy on their side of the board just like the attacker.
- **Ending the Run:** The battle plays out like a normal match. If either King is checkmated, that hero is eliminated, ending the run.

## 9. Balance knobs

| Knob | Starting Value | Description |
|---|---|---|
| `hero_base_movement` | 10 | Movement points per day. |
| `road_cost` | 0.5 | Movement cost on road tiles. |
| `forest_hill_cost` | 2.0 | Movement cost on rough terrain. |
| `sight_radius` | 5 | Tiles revealed around the hero. |
| `guard_scaling_rate` | 0.1 | Extra guard value added per day passed. |
| `camps_per_region` | 12 | Number of camps in each hero's start zone. |
| `ai_autoresolve_variance` | 0.15 | Seeded randomness / variance applied to off-screen AI camp fight resolution. |

## 10. Saves
The overworld state is saved at the start of each day and after every battle. The save file tracks the map seed, revealed fog, camp status, hero positions, army compositions, and decks. Losing the run deletes the save.

## 11. Mapping onto the code
- **`tc_world`:** A new crate or module to handle the map grid, fog of war, day loop, movement points, and overworld entities (heroes, camps).
- **`tc_run`:** Adapts to hold the global run state (permanent deck, hero army roster) instead of per-floor linear state.
- **`tc_core`:** Needs support for custom army setups (spawning arbitrary pieces instead of the standard chess setup) and handling the deployment phase.
- **`tc_ai`:** Needs an overworld AI to pick map destinations and pathfind, plus battle AI support for the deployment step.
- **`tc_game`:** Adds an overworld state/scene for map rendering, moving tokens, and transitions between the overworld view and the 3D battle board.

## 12. Milestones
1. **M-OW1: Playable 2-Hero Slice:** Basic map generation, hero movement, one camp type, simple auto-deploy for custom army setups (since armies start with 1 King + 3 Pawns), transition to battle, and army carry-over. No fog of war.
2. **M-OW2: Overworld Economy:** All camp types, pickups, permanent deck drafting, and 15-card hand building.
3. **M-OW3: AI Rival & Fog:** Implement fog of war, AI overworld behavior, and hero-vs-hero run ending.
4. **M-OW4: Art & Polish:** Implement overworld sprite sheet, custom battle terrain derived from map tiles, interactive manual deployment UI, and polish.

## 13. Open questions
- How do we handle a hero losing all pieces except the King? Are they effectively soft-locked, or should there be a desperate "free pawn" mechanic?
- Should camps regenerate guards if left alone for too long?
- How to ensure the battle board generation strictly reflects the map tile's intended difficulty/biome?
- Camp guard neutral tint: How should camp guard sprites be tinted (e.g., desaturated stone or weathered grey) to visually distinguish neutral defenders from true rival faction armies on the battle board?
- Fairness with 3+ heroes: regions are rotated copies on a square grid, which is exact only for 2 heroes (180°). With optimal pathfinding, regional path costs drift up to ~25 % apart at 3–4 heroes and more at 6+. Fix before enabling more than 2 heroes, e.g. rotate by rounding instead of truncating, use a hex grid, or balance per-region costs.
