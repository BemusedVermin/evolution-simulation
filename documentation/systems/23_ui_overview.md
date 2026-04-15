# UI System Overview

## 1. Purpose & Scope

The UI layer is the player's primary window into the Beast Evolution Game's simulation state. It serves three core functions:

1. **Situational Awareness**: Display the player's current location, available actions, and immediate environmental context (world map, encounter state, inventory).
2. **Information Access**: Provide readable, searchable access to discovered lore, catalogs (bestiary, materials, factions), and player history without exposing mechanical internals.
3. **Action Interface**: Enable player input (movement, dialog choices, crafting, trading, inspection) in a way that maps cleanly to simulation commands.

The UI layer does NOT contain simulation logic, does NOT persist game state, and does NOT affect determinism. UI state (scroll position, open tabs, filter selections) persists in a separate, non-canonical file that the engine ignores during simulation.

---

## 2. Rendering Modes

The game operates in two primary rendering modes, each serving a distinct play context and interaction model.

### 2.1 World Map Mode (Top-Down 2D)

**Purpose**: Traversal, strategic overview, faction/location discovery, ecological survey.

**Visual**:
- Top-down orthogonal or isometric view of the game world (archipelago).
- Tileset-based terrain (ocean, beach, grassland, forest, ruin, etc.).
- Creature symbols (abstract glyphs or small sprite icons) show population densities.
- Faction outposts marked with banners or architectural glyphs.
- Points of interest (ruins, resources, known NPCs) marked with icons.

**Interaction Model**:
- Click/WASD to move player avatar across map.
- Hover on creature/faction/location to inspect (triggers info panel overlay).
- Click on location to fast-travel (if known) or enter encounter (if creatures present).
- Keyboard shortcut to open any catalog (bestiary, materials, factions, events).
- Minimap in corner for global orientation.

**Transitions**:
- Encounter Mode: triggered by clicking on a location with creatures present, or by random encounter during travel.
- Catalog Screens: overlay on top of map; map remains visible/paused beneath.
- Deep-Sim Inspectors: overlay; map paused.

### 2.2 Encounter Mode (Fixed 2.5D Perspective)

**Purpose**: Tactical interaction, dialog, creature observation, detailed action resolution.

**Visual**:
- Fixed isometric or side-view camera focused on a region (100m × 100m).
- Creature models rendered at 1:1 or exaggerated scale (stylized).
- Terrain and objects (trees, rocks, ruins) in the environment.
- UI panels showing active creature, NPC names, action buttons, dialog log.
- Floating health/status indicators above creatures.

**Interaction Model**:
- Click creature to inspect or interact (triggers dialog/inspection menu).
- Action buttons: Move, Attack, Use Item, Cast Ability, Flee.
- Dialog tree with NPC faction members (choice-based conversation).
- Crafting/trading with NPCs happens in modal dialogs (not real-time).
- Text log at bottom showing events (creature action, damage, NPC speech).

**Transitions**:
- World Map Mode: click "leave area" button, or all creatures/NPCs flee/die.
- Catalog Screens: overlay; encounter paused.
- End of encounter: return to map at location or overworld if traveled far.

### 2.3 Activation Rules

**Enter World Map Mode**:
- Game start (initial spawn location).
- Exiting an encounter.
- Loading a saved game.

**Enter Encounter Mode**:
- Player clicks a map location with creatures present.
- Player triggers random encounter during fast-travel.
- Player enters a faction outpost (encounter with faction members).
- Scripted event (e.g., forced confrontation in lore).

**Pause/Resume**:
- World map mode: time advancing in background (creatures move, resources respawn), UI interactions pause map-time.
- Encounter mode: time paused while UI overlays open (catalogs, deep-sim inspectors); dialog choices pause; action selection pauses until confirmed.

---

## 3. UI Architecture

### 3.1 Rendering Decision: Retained-Mode

**Choice**: Retained-mode widget system.

**Rationale**:
- The game has complex, persistent UI state (catalog filters, open tabs, scroll positions, custom journal entries).
- Immediate-mode systems (egui-style) excel at throwaway dialogs and tools; they struggle with state persistence and undo/redo.
- A retained-mode widget tree allows us to declaratively define screens, bind to data sources, and re-render only when data changes.
- Determinism requirement: UI state must be fully serializable and restorable; retained-mode trees support this better than immediate-mode command streams.

**Alternative Considered**: Immediate-mode (egui, Dear ImGui)
- Pros: Rapid prototyping, no widget hierarchy to maintain.
- Cons: Harder to persist UI state, awkward with external data bindings, less suitable for complex catalog views and filters.

### 3.2 Widget Primitives

Core UI building blocks:

```
Widget {
    id: unique_id
    position: Rect
    
    // Event handlers
    on_click: Callable or null
    on_hover: Callable or null
    on_key: Callable or null
    
    // Visual
    background_color: Color or null
    text_content: String or null
    icon: SpriteID or null
}

Button {
    // extends Widget
    label: String
    is_enabled: bool
    on_pressed: Callable
}

List {
    // extends Widget
    items: Vec<ListItem>
    selected_index: int
    on_selection_change: Callable
    scroll_position: int
    // Supports filtering, searching, sorting
}

Card {
    // extends Widget
    title: String
    content: Vec<Widget>  // flexible nested layout
    is_expanded: bool
    on_expand: Callable
}

Chart {
    // extends Widget (for data visualization)
    chart_type: enum { LineChart, BarChart, PieChart, Timeline }
    data_source: DataBinding
    axis_labels: (String, String)  // x, y
}

Map {
    // extends Widget (specialized for world rendering)
    viewport: WorldRect
    zoom_level: float
    layer_visibility: map<LayerID, bool>
    hover_location: WorldLocation or null
    on_location_click: Callable
}

Dialog {
    // Modal overlay
    title: String
    content: Vec<Widget>
    buttons: Vec<Button>
    is_modal: bool
    on_close: Callable
}
```

### 3.3 Screen Graph

The screen hierarchy defines all major UI destinations and their relationships:

```
ScreenGraph {
    
    MainMenu {
        buttons: ["New Game", "Load Game", "Settings", "Credits", "Quit"]
        transitions_to: [WorldMapScreen, SettingsScreen, CreditsScreen]
    }
    
    WorldMapScreen (PRIMARY) {
        // Map widget showing archipelago
        components: {
            world_map: Map,
            location_inspector: Card,
            minimap: Map,
            action_bar: List<Button>,
            catalog_buttons: Vec<Button>,
            player_status: Card,
        }
        
        transitions_to: [
            EncounterScreen,
            BestiaryScreen,
            MaterialIndexScreen,
            EventFeedScreen,
            FactionListScreen,
            SettingsScreen,
        ]
    }
    
    EncounterScreen (PRIMARY) {
        // Tactical view of creature/NPC interaction
        components: {
            encounter_viewport: 3DViewport,
            creature_list: List<CreatureCard>,
            action_panel: List<Button>,
            dialog_panel: DialogBox,
            event_log: List<EventLogEntry>,
            status_indicators: Map<CreatureID, StatusCard>,
        }
        
        transitions_to: [
            WorldMapScreen,
            CreatureInspectorScreen,
            BestiaryScreen,
            DialogScreen,
        ]
    }
    
    // === CATALOG SCREENS ===
    
    BestiaryScreen {
        components: {
            entry_list: List<BestiaryEntry>,
            filter_panel: {
                discovered_only: bool,
                region_filter: Dropdown,
                threat_level_filter: Dropdown,
                sort_by: RadioGroup,
            },
            detail_view: Card,
            lineage_tree_button: Button,
        }
        
        transitions_to: [
            WorldMapScreen,
            EncounterScreen,
            LineageTreeScreen,
            CreatureInspectorScreen,
        ]
    }
    
    MaterialIndexScreen {
        components: {
            entry_list: List<MaterialEntry>,
            filter_panel: {
                material_type: Dropdown,
                discovered_only: bool,
                has_recipe: bool,
                sort_by: RadioGroup,
            },
            detail_view: Card,
            recipe_browser: List<RecipeReference>,
            market_data: Chart,
        }
        
        transitions_to: [
            WorldMapScreen,
            EncounterScreen,
            RecipeScreen,
        ]
    }
    
    EventFeedScreen {
        components: {
            timeline: Chart<Timeline>,
            event_list: List<EventIndexEntry>,
            filter_panel: {
                era_filter: RadioGroup,
                faction_filter: Dropdown,
                event_type_filter: MultiSelect,
                region_filter: Dropdown,
                locked_only: bool,
                sort_by: RadioGroup,
            },
            detail_view: Card,
            narrative_variants_panel: List<NarrativeVariant>,
        }
        
        transitions_to: [
            WorldMapScreen,
            EncounterScreen,
            FactionListScreen,
        ]
    }
    
    FactionListScreen {
        components: {
            faction_list: List<FactionEntry>,
            filter_panel: {
                region_filter: Dropdown,
                faction_type_filter: Dropdown,
                allied_only: bool,
                sort_by: RadioGroup,
            },
            detail_view: Card,
            relations_diagram: Chart<RelationsGraph>,
            reputation_meter: ProgressBar,
            interaction_log: List<Interaction>,
        }
        
        transitions_to: [
            WorldMapScreen,
            EncounterScreen,
            DialogScreen,
        ]
    }
    
    LineageTreeScreen {
        components: {
            tree_view: TreeWidget<LineageNode>,
            root_selection: Dropdown,
            detail_view: Card,
            phylogeny_chart: Chart<TreeChart>,
        }
        
        transitions_to: [
            BestiaryScreen,
            WorldMapScreen,
        ]
    }
    
    // === DEEP-SIM INSPECTORS ===
    
    CreatureInspectorScreen {
        // Detailed view of a single creature
        components: {
            creature_model: 3DViewport,
            phenotype_summary: Card,
            genetic_pedigree: Card,
            ability_list: List<Label>,  // shows Chronicler labels only, not primitives
            lineage_button: Button,
            encounter_history: List<Encounter>,
        }
        
        transitions_to: [
            BestiaryScreen,
            EncounterScreen,
            LineageTreeScreen,
        ]
    }
    
    RecipeScreen {
        components: {
            recipe_list: List<RecipeReference>,
            detail_view: Card,
            ingredient_list: List<MaterialEntry>,
            craft_button: Button,
        }
        
        transitions_to: [
            MaterialIndexScreen,
            EncounterScreen,
        ]
    }
    
    DialogScreen {
        // NPC conversation tree
        components: {
            npc_portrait: Image,
            npc_name: String,
            dialog_text: TextBlock,
            choice_buttons: Vec<Button>,
            faction_affiliation: String,
        }
        
        transitions_to: [
            EncounterScreen,
            FactionListScreen,
        ]
    }
    
    // === META SCREENS ===
    
    SettingsScreen {
        components: {
            rendering_options: OptionsList,
            audio_options: OptionsList,
            controls_rebind: ControlRebindPanel,
            accessibility_options: OptionsList,
        }
        
        transitions_to: [
            MainMenu,
            WorldMapScreen,
            EncounterScreen,
        ]
    }
    
    JournalScreen {
        // Player's custom notes & discoveries
        components: {
            journal_entries: List<JournalEntry>,
            creature_notes: Map<CreatureID, String>,
            faction_notes: Map<FactionID, String>,
            custom_naming_panel: NameCustomizer,
        }
        
        transitions_to: [
            WorldMapScreen,
            BestiaryScreen,
            FactionListScreen,
        ]
    }
    
    CrewManagementScreen {
        // (If applicable) manage party of creatures
        components: {
            party_list: List<CreatureCard>,
            add_remove_buttons: Vec<Button>,
            ability_loadout: List<Label>,
        }
        
        transitions_to: [
            WorldMapScreen,
            BestiaryScreen,
        ]
    }
}
```

### 3.4 Transitions & Overlays

**Overlay Dialogs** (non-blocking, map/encounter continues beneath):
- Location inspector (hover)
- Quick item tooltip
- Filter help panel
- Search result preview

**Modal Dialogs** (blocking, pause time):
- Confirm crafting
- Confirm trade
- Dialog choice selection
- Save/load prompt
- Settings confirmation

**Screen Transitions**:
- Fade in/out (0.2s) between major screens.
- Slide-in from side for catalog screens (opened from world map/encounter).
- Zoom animation for inspectors (creature/lineage detail).

---

## 4. Read Contract: UI Data Sources

The UI layer reads from exactly two sources:

### 4.1 Chronicler Query API (Formal Contract in System 09, Section 16)

The UI queries the Chronicler (System 09) for all user-facing strings, labels, and catalog data. All queries are **read-only snapshots** against the currently committed simulation tick.

**Formal Chronicler Query API** (see System 09, Section 16 for full spec):

```rust
ui_state.bestiary = chronicler.get_bestiary_entries(filter);
ui_state.bestiary_entry = chronicler.get_bestiary_entry(species_id);
ui_state.materials = chronicler.get_material_entries(filter);
ui_state.events = chronicler.get_event_feed(cursor, limit);
ui_state.factions = chronicler.get_faction_list();
ui_state.lineages = chronicler.get_lineage_tree(root, depth);

// Advanced queries for mechanical transparency
label_opt = chronicler.get_label_for_primitive_cluster(fingerprint);
labels_vec = chronicler.get_labels_for_species(species_id);
```

**Critical**: The UI never parses, manipulates, or displays mechanical data (primitives, genotypes, recipes, opinion vectors). The Chronicler is the **sole source of truth** for what the UI displays. UI queries return only:
- **Labels** (text, confidence, provenance): Names for species, materials, factions, events.
- **Catalogs** (entries with observation counts, aliases): Bestiary, materials, events, factions.
- **Phylogeny** (lineage trees): Evolutionary relationships.
- Never: primitive effect sets, genotype data, recipe yields, faction opinion vectors, AI state.

### 4.2 ECS Component Queries (Real-Time State)

The UI also queries the ECS system for real-time game state:
- Player position and inventory (from Transform, Inventory components).
- Creature positions, health, status effects (from Transform, Health, Status components).
- Encounter participants and action queue (from Encounter, ActionQueue components).
- Current location, weather, time of day (from World, WeatherSystem, TimeSystem).

**Invariant**: These queries are **structural** (which entities exist, their positions, their visible state), never **mechanical** (internal stats, AI decisions, simulation internals).

**Invariant 3.9 (reinforced)**: The UI never reads:
- Primitive effect sets
- Genotype data structures
- Recipe yield probabilities or crafting mechanics
- Faction opinion numeric vectors
- Creature AI state or decision-making data

Instead, the UI reads:
- Chronicler labels (lineage names, ability names)
- Chronicler catalogs (bestiary, materials, factions)
- Entity positions, health, visible status
- Player inventory (material stacks with Chronicler-resolved names)

This separation ensures **mechanical transparency**: players can read the rules and make decisions based on UI information alone, without needing to understand internal mechanical details.

---

## 5. Input Handling

### 5.1 Keyboard Bindings

```
== MOVEMENT ==
W/↑           Move forward
A/← or Q      Strafe left / Turn left
S/↓           Move backward
D/→ or E      Strafe right / Turn right
Space         Jump (encounter only) / Confirm (dialog)
Tab           Toggle world map / Encounter view

== INTERACTION ==
E             Interact with creature / NPC (inspect)
F             Fast-travel to known location (world map)
Esc           Open menu / Close current dialog
Ctrl+B        Toggle bestiary
Ctrl+M        Toggle material index
Ctrl+E        Toggle event feed
Ctrl+F        Toggle faction list
Ctrl+J        Toggle journal
Ctrl+S        Save game
Ctrl+L        Load game

== DIALOG ==
1-4           Select dialog choice (numbered options)
Space         Confirm selection / Continue text
Esc           Abandon dialog

== COMBAT / ENCOUNTER ==
1-9           Use ability 1-9
Right-click   Cancel action
Shift+↑↓      Cycle through targets
Esc           Open pause menu / Leave encounter
```

### 5.2 Mouse Bindings

```
Left-click    Select / Activate button / Open detail view
Right-click   Context menu (inspect, trade, dialog)
Scroll        Pan map / Scroll catalog list
Hover         Show tooltip / Highlight interactive element
Drag on map   Pan view
Drag on list  Reorder (if applicable)
```

### 5.3 Gamepad Support

- D-Pad: Navigate menu / Move creature
- Analog Stick (L): Free movement / Pan
- Analog Stick (R): Camera control (encounter)
- A / Cross: Confirm
- B / Circle: Cancel / Back
- X / Square: Secondary action (inspect)
- Y / Triangle: Tertiary action (menu)
- LB / L1: Previous tab / Category
- RB / R1: Next tab / Category
- Start: Pause / Open menu
- Back / Select: Quick-access (journal / last catalog)

### 5.4 Accessibility Hooks

- **Screen reader support**: All text labels and button actions announced.
- **High-contrast mode**: Toggle for high-contrast color scheme (UI follows WCAG AA guidelines).
- **Dyslexia-friendly font**: OpenDyslexic as optional font override.
- **Text size scaling**: 100% to 200% zoom on all UI text.
- **Colorblind modes**: Deuteranopia, Protanopia, Tritanopia simulations; icons use patterns in addition to color.
- **Controller-only mode**: No mouse required; all UI navigable via gamepad.
- **Pause-on-dialog**: Encounters pause automatically when NPC dialog opens.

---

## 6. Localization & Faction-Dialect Display

The UI displays three layers of naming for any in-world entity:

### 6.1 Canonical Names (Chronicler Labels)

- **Lineages**: Discovered label from pattern signature (e.g., "echolocation", "α-042").
- **Materials**: Thesaurus name or compositional name (e.g., "dense red mineral").
- **Factions**: Faction's self-name (e.g., "The Theocracy of the Drift Shores").
- **Events**: Canonical narrative (e.g., "The Cacogen Echo").

### 6.2 Faction-Coined Terms

Each faction (System 18) may coin alternative names for discovered labels. These are displayed in:
- Faction-specific NPC dialog.
- Faction bestiary or library UI (if accessed via faction).
- Faction journals or records.

**UI Display Rule**: Show canonical name in main catalogs (default bestiary); show faction-coined alternate as a tooltip or "Also known as" sub-text in faction-specific contexts.

### 6.3 Player Custom Names (Journal)

Players can rename creatures, factions, locations, and materials in their journal. These custom names:
- Persist in the player's save game (not in Chronicler).
- Display in the journal screen and in encounter dialogs (as a label override).
- Do NOT affect canonical catalogs (bestiary still shows Chronicler name).
- Do NOT affect NPC dialog (NPCs use canonical or faction-coined names).

**UI Display Rule**: Custom name shown in player's journal, and optionally in encounter UI as a subtitle or flavor note. Bestiary and catalogs remain canonical.

### 6.4 Language Localization

The game supports multiple human languages (English, French, Spanish, etc.). All UI text is localized:
- Button labels, menu text: localized via string table.
- Chronicler labels (bestiary names, material names, event descriptions): generated in English, but wrapped in localization keys for translation.
- Faction-coined terms: generated in English by faction language system, but wrapped in localization keys.
- Player custom names: stored as-is (player-authored); not localized.

**Determinism Note**: Localization does NOT affect simulation determinism. All internal name generation is done in English; localization is a UI-only transform applied at display time.

---

## 6.5 UI State vs. Sim State Boundary

**Invariant**: The UI never mutates sim state. All Chronicler queries are read-only snapshots; no query triggers side effects in the simulation.

### Sim State (Canonical, Deterministic)

The following is stored in the simulation save file and affects world state:
- **Entities**: Creatures, agents, settlements, factions, biome cells.
- **Phenotypes & Genotypes**: Channel values, body topology, genetic loci.
- **Populations**: Population counts per region.
- **Factions**: Names, members, settlements, technologies, cultural traits.
- **Chronicle Events**: Recorded significant events (extinctions, technologies, plagues).
- **Observation Counts**: Per-species count of sightings (derived from encounters).
- **Labels** (derived by Chronicler): Canonical names, confidence scores, first-seen ticks, provenance. These are computed from population-level patterns and stored in the simulation.

### UI State (Non-Canonical, Ephemeral)

The following persists in a separate `ui_state.json` file that the simulation engine **never reads**:
- **Camera Position**: Viewport on world map or encounter.
- **Open Tabs/Windows**: Which catalogs are currently visible.
- **Filter Selections**: Bestiary filter state (discovered_only, sort_by, etc.).
- **Scroll Positions**: Current scroll location in any list.
- **Zoom Level**: World map zoom, encounter camera angle.
- **Player-Authored Private Notes**: Custom creature names, faction notes, location annotations.
- **Uncollapsed/Collapsed Card States**: Which detail cards are expanded.
- **Search Query**: Current text in any search bar.

### Derived State: "Discovered" Status

A bestiary entry is marked as "discovered" by the UI when:

```
bestiary_entry.discovered = (sim_state.observation_count >= 1)
```

**Critical**: The "discovered" flag is **not stored** in the save file. It is **re-derived at load time** from the observation_count. This ensures that the simulation can never accidentally depend on UI-derived state; the flag is always computed from canonical sim state.

If a creature is sighted but later all creatures of that species are killed before the game is saved, the observation_count is still > 0, and the species remains "discovered" in the bestiary. (This is intentional: players learn about extinct species through records.)

---

## 7. Determinism Constraint: UI State Is Not Game State

**Principle**: The simulation's determinism must never depend on UI state.

### 7.1 UI State File

UI state (separate from GameState) includes:
- Scroll positions in catalog views
- Open/closed status of tabs
- Filter selections (discovered_only, sort_by, etc.)
- Zoom level on world map
- Encounter camera angle
- Recently viewed catalog entries
- Uncollapsed/collapsed card states
- Search query in search bar

All of this persists in a separate file (e.g., `ui_state.json`) that the engine **completely ignores** during simulation.

### 7.2 Enforced Separation

```rust
pub struct GameState {
    // Simulation state; affects determinism
    pub world: World,
    pub creatures: Vec<Creature>,
    pub factions: Vec<Faction>,
    pub chronicler: Chronicler,
    pub player: Player,
    // ... all mechanical and lore state
}

pub struct UIState {
    // UI state; NEVER affects determinism
    pub bestiary_scroll: int,
    pub bestiary_filter: BestiaryFilter,
    pub material_index_sort: SortOrder,
    pub open_tabs: Vec<TabID>,
    pub custom_creature_names: map<CreatureID, String>,
    pub recently_viewed: Vec<CatalogEntry>,
    pub world_map_zoom: float,
    pub encounter_camera_angle: float,
    // ... all UI ephemera
}

// Engine pseudocode
fn tick_simulation(game_state: &mut GameState, ui_state: &UIState) -> void {
    // Simulation NEVER reads ui_state
    // game_state updates deterministically
}

fn render_ui(game_state: &GameState, ui_state: &UIState) -> void {
    // UI rendering reads both game_state and ui_state
}

fn on_ui_input(ui_state: &mut UIState, input: Input) -> void {
    // UI input only modifies ui_state
    // Does NOT trigger any game_state changes
    // Exception: player movement/action in encounter feeds into game_state, but that's a deliberate game mechanic
}
```

### 7.3 Persistence

- **GameState**: Saved to `save.dat` (canonical game save).
- **UIState**: Saved to `ui_state.json` (loaded only for UI presentation, discarded when closing game or loading a different save).
- **Recovery**: If `ui_state.json` is lost or corrupted, the game restores default UI settings (bestiary unfiltered, all tabs closed, etc.) and continues normally.

---

## 8. Mockup Inventory

The following SVG mockups exist in `/ui/mockups/`:

### Core Screens

- **core/main_menu.svg**: Title screen with New Game, Load, Settings, Quit.
- **core/world_map.svg**: Archipelago map with location icons, minimap, action bar, player status.
- **core/encounter_screen.svg**: Tactical view with creature list, action panel, dialog box, event log.
- **core/pause_menu.svg**: Overlay menu (Resume, Settings, Save, Load, Quit).

### Catalog Screens

- **catalog/bestiary.svg**: Entry list with filter panel, detail view, lineage tree button.
- **catalog/material_index.svg**: Material list with filters, detail view, recipe browser, market chart.
- **catalog/event_feed.svg**: Timeline chart, event list with filters, narrative variants panel.
- **catalog/faction_list.svg**: Faction list with relations diagram, reputation meter, interaction log.
- **catalog/lineage_tree.svg**: Tree widget showing phylogeny, root selection dropdown, detail view.

### Deep-Sim Inspectors

- **deepsim/creature_inspector.svg**: 3D creature model, phenotype summary, genetic pedigree, ability list, encounter history.
- **deepsim/recipe_screen.svg**: Recipe detail, ingredient list, craft button.
- **deepsim/dialog_screen.svg**: NPC portrait, dialog text, choice buttons, faction affiliation.

### Meta Screens

- **meta/settings.svg**: Rendering, audio, controls, accessibility options.
- **meta/journal.svg**: Player notes, creature notes, faction notes, custom naming panel.
- **meta/crew_management.svg**: (Optional) party list, ability loadout, add/remove buttons.

**Note**: Mockups are SVG for easy iteration and localization. Final rendered UI will use a combination of:
- Hand-authored sprites (buttons, icons, portraits).
- Procedurally generated charts (timeline, relations graph).
- 3D creature models (creature inspector, encounter view).
- Font-rendered text (all labels, dialog).

---

## 9. Tradeoff Matrix

### 9.1 Immediate-Mode vs. Retained-Mode

| Dimension | Immediate-Mode (egui) | Retained-Mode | Winner |
|---|---|---|---|
| **Prototyping Speed** | Fast (imperative code) | Slower (declare structure) | Immediate |
| **State Persistence** | Awkward (state not built-in) | Natural (widget tree has state) | Retained |
| **Data Binding** | Manual synchronization | Automatic (reactivity) | Retained |
| **Complex Filters/Tabs** | Verbose (many if-branches) | Clean (data-driven) | Retained |
| **Undo/Redo** | Fragile (command history) | Easier (widget tree snapshots) | Retained |
| **Performance** | Fast (single-pass) | Higher overhead (tree traversal) | Immediate |
| **Code Clarity** | Simple (read top-to-bottom) | More structure (boilerplate) | Immediate |

**Winner**: Retained-Mode. The game's persistent UI state, complex catalogs, and determinism requirements favor structure and data binding.

### 9.2 Widget System: egui vs. SDL vs. Custom

| Dimension | egui | Raw SDL Widgets | Custom Layer |
|---|---|---|---|
| **Development Time** | Low (battle-tested) | High (rebuild everything) | Medium (custom, but targetable) |
| **Flexibility** | Medium (opinionated) | High (no constraints) | High (if well-designed) |
| **Performance** | Good (optimized) | Variable (depends on impl) | Good (if optimized) |
| **Integration** | Easy (Rust + egui crate) | Moderate (SDL + custom glue) | High effort (ground-up) |
| **Maintainability** | High (community support) | Medium (your code) | Low (fragile custom code) |
| **Feature Richness** | Rich (built-in widgets) | Minimal (you add) | Custom (what you write) |

**Winner**: Custom Layer built on SDL2. Rationale:
- egui is primarily for immediate-mode, which we rejected.
- Raw SDL is too low-level and verbose for the complexity we need.
- A thin custom retained-mode layer on SDL provides the right balance: full control, deterministic rendering, clean data binding, and access to 3D rendering (encounters, creature models).
- We can borrow patterns from egui and Dear ImGui for widget design while maintaining full control.

### 9.3 Localization Strategy

| Dimension | String Tables (gettext-style) | YAML/JSON with Keys | Dynamic String Generation |
|---|---|---|---|
| **Translator Workflow** | Industry standard | Simple to edit | Requires code changes |
| **Coverage** | All strings covered | All strings covered | Partial (generated strings) |
| **Maintenance** | High (each language file) | Medium (keys align) | Low (generated) |
| **Culture-Specific Formatting** | Full support (plurals, genders) | Basic (keys only) | Complex (logic-heavy) |
| **Concatenation Issues** | Fragile (word order) | Fragile | Very fragile |

**Winner**: Hybrid approach:
- Core UI text (buttons, labels, menu items): String tables (gettext-style).
- Chronicler labels (lineage names, material names, event descriptions): JSON keys mapping generated names to localized strings.
- Faction language: Generated in English, wrapped in i18n keys; translator provides faction-specific translations.
- Player custom names: Stored as-is (player-authored); never localized.

This balances translator workflow with the need for generated/emergent content.

---

## 10. Research Anchors

### 10.1 Usability Heuristics (Nielsen, 1994)

The UI design follows Nielsen's 10 usability heuristics:

1. **Visibility of system status**: World map shows player position, health, time. Encounter shows creature status, action queue.
2. **Match between system and real world**: Archipelago metaphor, creature icons, faction banners. Language matches player expectations.
3. **User control and freedom**: Escape key cancels dialogs. Journal allows custom renaming. Filters let players explore catalogs their way.
4. **Error prevention and recovery**: Confirm before crafting/trading. Save slots. UI state saved separately (doesn't break game).
5. **Help and documentation**: Tooltips on all buttons. Help mode (F1). In-game encyclopedia (lore fragments, discoveries).
6. **Flexibility and efficiency**: Keyboard shortcuts for frequent actions. Customizable controls. Quick-access to recent catalogs.
7. **Aesthetic and minimalist design**: UI color palette matches world tone. Unused elements hidden. Compact layout.
8. **Error messages**: Clear, actionable. "You don't have this material" vs. generic error.
9. **Error recovery**: Undo last craft (if applicable). Return to map from encounter with inventory intact.
10. **Consistency**: Button styles, icon meanings, menu structure consistent across all screens.

### 10.2 Affordance Theory (Gibson, 1977; Norman, 1988)

UI elements signal their affordances:

- **Buttons**: Raised/shadowed appearance signals "pressable"; hover state confirms interactivity.
- **Lists**: Scrollbar signals "many items"; selection highlight shows current item.
- **Drag handles**: Textured appearance signals "draggable" (if applicable).
- **Links/Text**: Blue color + underline signal "clickable" (standard web conventions).
- **Cards**: Border and shadow signal "inspectable detail"; expand arrow signals "more content".
- **Slider**: Horizontal bar + thumb signal "adjustable value".
- **Text input**: Border and cursor signal "editable".

Colors, shadows, icons, and text labels all work together to make interaction affordances obvious.

### 10.3 Cataloging UX: Dwarf Fortress & RimWorld

Both games present deep emergent worlds to players through sophisticated, information-dense catalogs.

**Dwarf Fortress (Legends mode)** lessons:
- **Searchable event logs**: Players can search by participant, location, date. This drives discovery of lore.
- **Layered detail**: Top-level event list; clicking reveals narrative variants and contradictions.
- **Naming ambiguity**: "the goblin" vs. "Urist McSoldier" — emergent identity through repeated observation.
- **Lineage/genealogy tracking**: Players trace family trees and faction histories.

**RimWorld** lessons:
- **Faction relations with numeric transparency**: Allied, neutral, hostile. Why? Quests, raids, reputation changes. Transparency enables player agency.
- **Creature library as encyclopedia**: Entries unlock with discovery, not pre-authored. Player learns as they encounter.
- **Event timeline as player history**: Raids, deaths, births, trade recorded chronologically. Player constructs narrative from these atoms.
- **Flexibility in naming**: Creatures named automatically; player renames at will. Both names coexist in different contexts.

**Our adoption**:
- Chronicler catalogs (bestiary, materials, factions, events) are searchable, filterable, layered.
- Narrative variants (like Dwarf Fortress) show contradictory accounts; player triangulates truth.
- Lineage tree (like genealogy) shows phylogenic history.
- Faction relations shown with numeric reputation and interaction log (like RimWorld).
- Player custom naming coexists with canonical names.
- Event timeline visible as a chart; deep-dive into individual events available.

---

## 11. Design Decisions Deferred

The following aspects are explicitly **future work** and not specified in this overview:

### 11.1 Art Style

- **Sprite vs. Voxel vs. 3D**: Rendered creature models in encounter mode not yet decided. Concept art needed.
- **Color Palette**: World tone and faction-specific visual languages (colors, patterns) not finalized.
- **UI Theme**: Dark vs. light, gritty vs. clean aesthetic. Mockups are grayscale; final colors TBD.

### 11.2 Typography

- **Font Families**: Primary font (UI labels, dialog), monospace font (code/technical text), decorative font (faction names, titles).
- **Font Sizing**: Exact point sizes for different UI hierarchy levels.
- **Font Rendering**: Anti-aliasing, hinting, ligatures.

### 11.3 Iconography

- **Icon Set**: Set of action icons (move, attack, use item, trade, etc.), faction emblems, status indicators, resource icons.
- **Icon Style**: Detailed realism vs. flat design vs. pixel art.
- **Accessibility Icons**: Symbols for abilities that don't have visual metaphors (e.g., "sense of smell").

### 11.4 Animation

- **Transition Animations**: Fade, slide, zoom — duration, easing curves.
- **Idle Animations**: Creature breathing, flickering elements, background parallax.
- **Feedback Animations**: Button press, damage number float, health bar decrease.

### 11.5 Sound & Music

- **UI Sounds**: Click, hover, dialog open, item pickup.
- **Ambient Music**: World map, encounter, faction-specific themes.
- **Voice Acting**: (Optional) NPC dialog voice lines; faction-specific accents.

### 11.6 Accessibility Testing

- **WCAG 2.1 AA Compliance**: Detailed audit deferred until visual design is finalized.
- **User Testing with Disabilities**: Real-world testing with screen reader users, colorblind users, etc.

---

## 12. Summary & Next Steps

This overview specifies the UI system's architecture, data flow, interaction model, and constraints. The system is designed to:

1. **Read only from Chronicler catalogs and ECS state**, preserving the separation of UI presentation from mechanical data.
2. **Persist UI state separately**, ensuring that UI preferences never affect simulation determinism.
3. **Support multiple rendering modes** (world map, encounter), each with distinct affordances.
4. **Implement retained-mode widgets** on a custom SDL2 layer for state persistence and data binding.
5. **Follow usability heuristics** (Nielsen) and affordance theory (Gibson/Norman).
6. **Enable rich catalog exploration** inspired by Dwarf Fortress and RimWorld.
7. **Support localization, faction dialects, and player customization** without breaking canonical naming.

Next steps include:
- **Finalize mockups** for each screen in color.
- **Implement custom retained-mode widget layer** on SDL2.
- **Connect UI to Chronicler Query API** and ECS.
- **Implement input handling** (keyboard, mouse, gamepad).
- **User testing** with prototypes to validate usability.
- **Accessibility audit** against WCAG 2.1 AA.
- **Deferred decisions** on art style, typography, iconography, animation.
